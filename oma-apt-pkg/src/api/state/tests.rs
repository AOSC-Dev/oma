use super::*;
use crate::api::Control;
use crate::SourceKind;

fn control(name: &str, version: &str, arch: &str, description: &str) -> Control {
    crate::deb::parse_control_entry(&format!(
        "Package: {name}\nVersion: {version}\nArchitecture: {arch}\nDescription: {description}\n\n"
    ))
    .unwrap()
}

fn source(arch: &str, suite: &str) -> Source {
    Source {
        kind: SourceKind::Repository,
        base_url: "file:/repo".to_string(),
        suite: suite.to_string(),
        component: Some("main".to_string()),
        arch: Some(arch.to_string()),
    }
}

fn state(native: &str, archs: &[&str]) -> AptState {
    AptState {
        native: Arch::new(native),
        architectures: archs.iter().map(|arch| Arch::new(*arch)).collect(),
        groups: HashMap::new(),
        package_order: HashMap::new(),
        installed: HashMap::new(),
    }
}

fn add(state: &mut AptState, name: &str, version: &str, arch: &str) {
    add_pair(
        state,
        control(name, version, arch, "test"),
        source(arch, "stable"),
    );
}

fn add_pair(state: &mut AptState, control: Control, source: Source) {
    let version_arch = Arch::new(
        control
            .architecture
            .clone()
            .unwrap_or_else(|| state.native.to_string()),
    );
    let package_arch = if version_arch.is_all() {
        state.native.clone()
    } else {
        version_arch.clone()
    };
    state.insert_version(
        &control.package.clone(),
        package_arch,
        Version {
            control,
            sources: vec![source],
            arch: version_arch,
        },
    );
}

fn versions(package: &Package<'_>) -> Vec<(String, String)> {
    package
        .versions()
        .iter()
        .map(|version| {
            (
                version.version().unwrap_or_default().to_string(),
                version.arch().as_str().to_string(),
            )
        })
        .collect()
}

#[test]
fn all_is_a_version_flag_not_a_package_arch() {
    let mut state = state("amd64", &["amd64", "i386"]);
    add(&mut state, "foo", "1", "amd64");
    add(&mut state, "foo", "2", "i386");
    add(&mut state, "foo", "3", "all");

    let group = state.group("foo").unwrap();
    let mut arches: Vec<_> = group.packages().map(|pkg| pkg.arch().to_string()).collect();
    arches.sort();
    assert_eq!(arches, vec!["amd64", "i386"]);

    let native = state.package("foo", &Arch::new("amd64")).unwrap();
    assert_eq!(
        versions(&native),
        vec![
            ("1".to_string(), "amd64".to_string()),
            ("3".to_string(), "all".to_string())
        ]
    );
    let foreign = state.package("foo", &Arch::new("i386")).unwrap();
    assert_eq!(
        versions(&foreign),
        vec![("2".to_string(), "i386".to_string())]
    );
}

#[test]
fn resolve_uses_apt_architecture_views() {
    let mut state = state("amd64", &["amd64", "i386"]);
    add(&mut state, "foo", "1", "amd64");
    add(&mut state, "foo", "2", "i386");
    add(&mut state, "foo", "3", "all");
    add(&mut state, "bar", "1", "i386");
    add(&mut state, "baz", "1", "all");

    assert_eq!(
        state.resolve(vec!["foo".into()]).resolved[0]
            .key
            .arch
            .as_str(),
        "amd64"
    );
    for query in ["foo:native", "foo:any", "foo:all", "foo:amd64"] {
        let resolution = state.resolve(vec![query.into()]);
        assert_eq!(resolution.resolved[0].key.arch.as_str(), "amd64", "{query}");
    }
    let foreign = state.resolve(vec!["foo:i386".into()]);
    assert_eq!(foreign.resolved[0].key.arch.as_str(), "i386");

    // A package only a foreign architecture has falls back for a plain
    // query, but `:native`/`:all` do not.
    let fallback = state.resolve(vec!["bar".into()]);
    assert_eq!(fallback.resolved[0].key.arch.as_str(), "i386");
    for query in ["bar:native", "bar:all"] {
        assert_eq!(state.resolve(vec![query.into()]).no_match, vec![query]);
    }

    // An `all`-only package belongs to the native package.
    let all_only = state.resolve(vec!["baz".into()]);
    assert_eq!(all_only.resolved[0].key.arch.as_str(), "amd64");
    assert_eq!(all_only.resolved[0].versions[0].arch().as_str(), "all");
    assert_eq!(
        state.resolve(vec!["baz:i386".into()]).no_match,
        vec!["baz:i386"]
    );
}

#[test]
fn version_and_branch_queries_filter_one_package() {
    let mut state = state("amd64", &["amd64"]);
    add(&mut state, "foo", "1", "amd64");
    add_pair(
        &mut state,
        control("foo", "2", "amd64", "preview"),
        source("amd64", "preview"),
    );

    let exact = state.resolve(vec!["foo=1".into()]);
    assert_eq!(exact.resolved[0].versions.len(), 1);
    assert_eq!(exact.resolved[0].versions[0].version(), Some("1"));

    let branch = state.resolve(vec!["foo/preview".into()]);
    assert_eq!(branch.resolved[0].versions.len(), 1);
    assert_eq!(branch.resolved[0].versions[0].version(), Some("2"));
}

#[test]
fn local_deb_resolves_only_its_own_version() {
    let mut state = state("amd64", &["amd64"]);
    add(&mut state, "foo", "1", "amd64");
    add(&mut state, "foo", "2", "amd64");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("foo_9_amd64.deb");
    std::fs::write(
        &path,
        crate::deb::test_util::build_deb(
            "Package: foo\nVersion: 9\nArchitecture: amd64\nDescription: local\n",
        ),
    )
    .unwrap();
    state.insert_deb(&path).unwrap();

    let resolution = state.resolve(vec![path.to_string_lossy().into_owned()]);
    assert_eq!(resolution.resolved.len(), 1);
    let versions = &resolution.resolved[0].versions;
    assert_eq!(versions.len(), 1, "only the .deb's version is selected");
    assert_eq!(versions[0].version(), Some("9"));
}

#[test]
fn local_deb_keeps_its_control_when_the_repo_has_the_same_version() {
    let mut state = state("amd64", &["amd64"]);
    add(&mut state, "foo", "1", "amd64");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("foo_1_amd64.deb");
    std::fs::write(
        &path,
        crate::deb::test_util::build_deb(
            "Package: foo\nVersion: 1\nArchitecture: amd64\nDescription: local control\n",
        ),
    )
    .unwrap();
    state.insert_deb(&path).unwrap();

    let resolution = state.resolve(vec![path.to_string_lossy().into_owned()]);
    let versions = &resolution.resolved[0].versions;
    assert_eq!(versions.len(), 1);
    assert_eq!(
        versions[0].control.description.as_deref(),
        Some("local control")
    );
    assert!(versions[0].sources.iter().all(Source::is_local_deb));

    // Mixing a path with a name in one invocation: the name query must see
    // the repository record only, never the inserted local one.
    let mixed = state.resolve(vec!["foo".into(), path.to_string_lossy().into_owned()]);
    assert_eq!(mixed.resolved.len(), 2);
    let repo = &mixed.resolved[0];
    assert_eq!(repo.versions.len(), 1);
    assert!(
        repo.versions[0]
            .sources
            .iter()
            .all(|source| !source.is_local_deb())
    );
    assert_eq!(
        repo.versions[0].control.description.as_deref(),
        Some("test")
    );
    assert!(
        mixed.resolved[1]
            .versions
            .iter()
            .all(|version| version.is_local_deb())
    );
}

#[test]
fn distinct_local_debs_with_the_same_identity_stay_separate() {
    let mut state = state("amd64", &["amd64"]);

    let dir = tempfile::tempdir().unwrap();
    let one = dir.path().join("foo_1_amd64.deb");
    let two = dir.path().join("foo_1_amd64-copy.deb");
    std::fs::write(
        &one,
        crate::deb::test_util::build_deb(
            "Package: foo\nVersion: 1\nArchitecture: amd64\nDescription: first local\n",
        ),
    )
    .unwrap();
    std::fs::write(
        &two,
        crate::deb::test_util::build_deb(
            "Package: foo\nVersion: 1\nArchitecture: amd64\nDescription: second local\n",
        ),
    )
    .unwrap();
    state.insert_deb(&one).unwrap();
    state.insert_deb(&two).unwrap();

    for (path, description) in [(&one, "first local"), (&two, "second local")] {
        let resolution = state.resolve(vec![path.to_string_lossy().into_owned()]);
        let versions = &resolution.resolved[0].versions;
        assert_eq!(versions.len(), 1);
        assert_eq!(
            versions[0].control.description.as_deref(),
            Some(description)
        );
    }
}

#[test]
fn local_deb_does_not_shadow_an_indexed_architecture() {
    let mut state = state("amd64", &["amd64", "i386"]);
    add(&mut state, "foo", "1", "i386");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("foo_2_amd64.deb");
    std::fs::write(
        &path,
        crate::deb::test_util::build_deb(
            "Package: foo\nVersion: 2\nArchitecture: amd64\nDescription: local native\n",
        ),
    )
    .unwrap();
    state.insert_deb(&path).unwrap();

    // The local native record is not an indexed native package: a plain query
    // falls back to the indexed i386 package instead of selecting it.
    let plain = state.resolve(vec!["foo".into()]);
    assert_eq!(plain.resolved.len(), 1);
    assert_eq!(plain.resolved[0].key.arch.as_str(), "i386");
    assert_eq!(plain.resolved[0].versions[0].version(), Some("1"));
    assert!(
        plain.resolved[0]
            .versions
            .iter()
            .all(|version| !version.is_local_deb())
    );

    // An explicit native query must not match the local-only native arch.
    assert_eq!(
        state.resolve(vec!["foo:native".into()]).no_match,
        vec!["foo:native"]
    );
}

#[test]
fn repository_suite_named_local_deb_is_still_a_repository() {
    let mut state = state("amd64", &["amd64"]);
    add_pair(
        &mut state,
        control("foo", "1", "amd64", "from a suite named local-deb"),
        source("amd64", "local-deb"),
    );

    let resolution = state.resolve(vec!["foo".into()]);
    assert_eq!(resolution.resolved.len(), 1);
    let versions = &resolution.resolved[0].versions;
    assert_eq!(versions.len(), 1);
    assert!(!versions[0].is_local_deb());
    assert_eq!(versions[0].sources[0].kind, SourceKind::Repository);
}
