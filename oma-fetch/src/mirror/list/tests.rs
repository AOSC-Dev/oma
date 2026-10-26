use super::*;
use crate::mirror::{MirrorCandidate, MirrorError, MirrorTarget, MirrorTransport, TargetKind};

const SOURCE: &str = "mirror+file:/etc/apt/mirrors.list";
const INRELEASE: &str = "mirror+file:/etc/apt/mirrors.list/InRelease";

fn list(text: &str) -> MirrorList {
    MirrorList::parse(SOURCE, text, false).unwrap()
}

fn seeded_list(text: &str, seed: u64) -> MirrorList {
    let mut list = list(text);
    list.seed = seed;
    list
}

fn urls(list: &MirrorList, target: &MirrorTarget<'_>) -> Vec<String> {
    list.resolve(SOURCE, INRELEASE, target)
        .unwrap()
        .into_iter()
        .map(|candidate| candidate.url)
        .collect()
}

#[test]
fn remote_lists_reject_local_mirrors() {
    let text = "http://m1.example.com/\nfile:/srv/repo\n";
    let list = MirrorList::parse("mirror+https://host/list", text, true).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list.entries[0].uri, "http://m1.example.com/");

    let err = MirrorList::parse("mirror+https://host/list", "file:/srv/repo\n", true)
        .expect_err("remote list with only file mirrors must fail");
    assert!(matches!(err, MirrorError::NoSupportedMirrors { .. }));
}

#[test]
fn skips_unsupported_transports() {
    let list =
        list("ftp://m1.example.com/\ncopy:/m2/\nmirror+http://m3/list\nhttp://ok.example.com/\n");

    assert_eq!(list.len(), 1);
    assert_eq!(list.entries[0].uri, "http://ok.example.com/");
}

#[test]
fn rejects_empty_lists() {
    let err = MirrorList::parse(SOURCE, "# only comments\n\n", false)
        .expect_err("an empty mirror list must be rejected");
    assert!(matches!(err, MirrorError::EmptyList { .. }));
}

#[test]
fn filters_by_arch_component_language_and_type() {
    let list = seeded_list(
        "\
         http://plain.example.com/\n\
         http://amd.example.com/\tarch:amd64\n\
         http://arm.example.com/\tarch:arm64\n\
         http://index.example.com/\ttype:index\n\
         http://deb.example.com/\ttype:deb\n\
         http://main.example.com/\tcomponent:main\n\
         http://contrib.example.com/\tcomponent:contrib\n\
         http://zh.example.com/\tlang:zh_CN\n\
         http://en.example.com/\tlang:en\n",
        1,
    );

    // Package-style target: architecture + component + language + type.
    let package_target = MirrorTarget {
        arch: Some("amd64"),
        component: Some("main"),
        language: Some("zh_CN"),
        kind: TargetKind::File("deb"),
        ..Default::default()
    };
    let selected = urls(&list, &package_target);

    // Untagged mirrors serve every request.
    assert!(selected.contains(&"http://plain.example.com/InRelease".to_string()));
    // arch:amd64 matches, arch:arm64 does not.
    assert!(selected.contains(&"http://amd.example.com/InRelease".to_string()));
    assert!(!selected.contains(&"http://arm.example.com/InRelease".to_string()));
    // type:deb matches a package request, type:index does not.
    assert!(selected.contains(&"http://deb.example.com/InRelease".to_string()));
    assert!(!selected.contains(&"http://index.example.com/InRelease".to_string()));
    // component:main matches, component:contrib does not.
    assert!(selected.contains(&"http://main.example.com/InRelease".to_string()));
    assert!(!selected.contains(&"http://contrib.example.com/InRelease".to_string()));
    // lang:zh_CN matches, lang:en does not.
    assert!(selected.contains(&"http://zh.example.com/InRelease".to_string()));
    assert!(!selected.contains(&"http://en.example.com/InRelease".to_string()));

    // Index-style target: type:index and no component/language filter.
    let index_target = MirrorTarget {
        arch: Some("amd64"),
        kind: TargetKind::Index,
        ..Default::default()
    };
    let selected = urls(&list, &index_target);

    assert!(selected.contains(&"http://index.example.com/InRelease".to_string()));
    assert!(!selected.contains(&"http://deb.example.com/InRelease".to_string()));
    assert!(selected.contains(&"http://main.example.com/InRelease".to_string()));
    assert!(selected.contains(&"http://contrib.example.com/InRelease".to_string()));
    assert!(selected.contains(&"http://zh.example.com/InRelease".to_string()));
    assert!(selected.contains(&"http://en.example.com/InRelease".to_string()));

    // Unspecified type does not filter type-tagged entries.
    let unspecified = MirrorTarget {
        kind: TargetKind::Unspecified,
        ..Default::default()
    };
    let selected = urls(&list, &unspecified);
    assert!(selected.contains(&"http://index.example.com/InRelease".to_string()));
    assert!(selected.contains(&"http://deb.example.com/InRelease".to_string()));
}

#[test]
fn matches_suite_and_codename_like_apt() {
    let list = seeded_list(
        "\
         http://suite.example.com/\tsuite:bookworm\n\
         http://suite-other.example.com/\tsuite:trixie\n\
         http://codename.example.com/\tcodename:bookworm\n\
         http://codename-other.example.com/\tcodename:trixie\n\
         http://release.example.com/\trelease:bookworm\n",
        1,
    );

    // Target-Release (sources.list suite) matches every spelling of the
    // release, because suite/codename tags are also recorded under
    // Release. Conflicting values are excluded.
    let release = MirrorTarget {
        release: Some("bookworm"),
        ..Default::default()
    };
    let selected = urls(&list, &release);
    assert_eq!(selected.len(), 3);
    assert!(selected.contains(&"http://suite.example.com/InRelease".to_string()));
    assert!(selected.contains(&"http://codename.example.com/InRelease".to_string()));
    assert!(selected.contains(&"http://release.example.com/InRelease".to_string()));
    assert!(!selected.contains(&"http://suite-other.example.com/InRelease".to_string()));
    assert!(!selected.contains(&"http://codename-other.example.com/InRelease".to_string()));

    // Target-Suite excludes suite:trixie, but entries without a Suite
    // tag still match (a missing tag never filters a mirror out).
    let suite = MirrorTarget {
        suite: Some("bookworm"),
        ..Default::default()
    };
    let selected = urls(&list, &suite);
    assert!(selected.contains(&"http://suite.example.com/InRelease".to_string()));
    assert!(!selected.contains(&"http://suite-other.example.com/InRelease".to_string()));
    assert!(selected.contains(&"http://codename.example.com/InRelease".to_string()));
    assert!(selected.contains(&"http://release.example.com/InRelease".to_string()));

    // Target-Codename behaves the same way for codename: tags.
    let codename = MirrorTarget {
        codename: Some("bookworm"),
        ..Default::default()
    };
    let selected = urls(&list, &codename);
    assert!(selected.contains(&"http://codename.example.com/InRelease".to_string()));
    assert!(!selected.contains(&"http://codename-other.example.com/InRelease".to_string()));
    assert!(selected.contains(&"http://suite.example.com/InRelease".to_string()));
    assert!(selected.contains(&"http://release.example.com/InRelease".to_string()));

    // A target value that no tag carries filters the entry out.
    let other = MirrorTarget {
        release: Some("unstable"),
        ..Default::default()
    };
    assert!(list.resolve(SOURCE, INRELEASE, &other).is_err());
}

#[test]
fn orders_by_priority_and_keeps_untagged_last() {
    let list = seeded_list(
        "\
         http://p2.example.com/\tpriority:2\n\
         http://p1.example.com/\tpriority:1\n\
         http://p9.example.com/\tpriority:9\n\
         http://untagged.example.com/\n",
        7,
    );

    let selected = list
        .resolve(SOURCE, INRELEASE, &MirrorTarget::default())
        .unwrap();

    assert_eq!(selected[0].url, "http://p1.example.com/InRelease");
    assert_eq!(selected[1].url, "http://p2.example.com/InRelease");
    assert_eq!(selected[2].url, "http://p9.example.com/InRelease");
    assert_eq!(selected[3].url, "http://untagged.example.com/InRelease");
    assert_eq!(selected[0].priority, 1);
    assert_eq!(selected[3].priority, u64::MAX);
}

#[test]
fn equal_priority_order_is_stable_within_one_list() {
    let text = "http://a.example.com/\nhttp://b.example.com/\nhttp://c.example.com/\n";

    let first = seeded_list(text, 42)
        .resolve(SOURCE, INRELEASE, &MirrorTarget::default())
        .unwrap();
    let second = seeded_list(text, 42)
        .resolve(SOURCE, INRELEASE, &MirrorTarget::default())
        .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.len(), 3);
}

#[test]
fn errors_when_no_mirror_matches() {
    let list = list("http://amd.example.com/\tarch:amd64\n");
    let target = MirrorTarget {
        arch: Some("arm64"),
        ..Default::default()
    };

    let err = list
        .resolve(SOURCE, INRELEASE, &target)
        .expect_err("no matching mirror must be an error");
    assert!(matches!(err, MirrorError::NoCandidates { .. }));
}

#[test]
fn resolves_urls_against_mirror_bases() {
    let list = seeded_list(
        "\
         http://m1.example.com/debian/\tpriority:1\n\
         file:/srv/repo\tpriority:2\n",
        1,
    );
    let target = MirrorTarget {
        arch: Some("amd64"),
        kind: TargetKind::Index,
        ..Default::default()
    };
    let url = "mirror+file:/etc/apt/mirrors.list/dists/bookworm/main/binary-amd64/Packages.xz";

    let selected = list.resolve(SOURCE, url, &target).unwrap();

    assert_eq!(
        selected[0],
        MirrorCandidate {
            url: "http://m1.example.com/debian/dists/bookworm/main/binary-amd64/Packages.xz".into(),
            transport: MirrorTransport::Http,
            priority: 1,
        }
    );
    assert_eq!(
        selected[1],
        MirrorCandidate {
            url: "file:/srv/repo/dists/bookworm/main/binary-amd64/Packages.xz".into(),
            transport: MirrorTransport::File,
            priority: 2,
        }
    );
}

#[test]
fn resolve_accepts_trailing_slashes() {
    let list = list("http://m1.example.com/debian/\n");
    let source = "mirror+file:/etc/apt/mirrors.list/";
    let url = "mirror+file:/etc/apt/mirrors.list/dists/bookworm/InRelease";

    let selected = list.resolve(source, url, &MirrorTarget::default()).unwrap();

    assert_eq!(
        selected[0].url,
        "http://m1.example.com/debian/dists/bookworm/InRelease"
    );
}

#[test]
fn rejects_urls_outside_the_source() {
    let list = list("http://m1.example.com/debian/\n");

    let err = list
        .resolve(
            "mirror+file:/etc/apt/mirrors.list",
            "mirror+file:/etc/apt/mirrors.list-other/InRelease",
            &MirrorTarget::default(),
        )
        .expect_err("a non-boundary prefix must not match");
    assert!(matches!(err, MirrorError::UrlMismatch { .. }));

    let err = list
        .resolve(
            "mirror+file:/etc/apt/mirrors.list",
            "mirror+file:/another/list/InRelease",
            &MirrorTarget::default(),
        )
        .expect_err("an unrelated URL must not match");
    assert!(matches!(err, MirrorError::UrlMismatch { .. }));
}

#[test]
fn candidates_returns_mirror_bases() {
    let list = list("http://m1.example.com/debian/\n");

    let candidates = list.candidates(&MirrorTarget::default()).unwrap();

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].url, "http://m1.example.com/debian/");
    assert_eq!(candidates[0].transport, MirrorTransport::Http);
}
