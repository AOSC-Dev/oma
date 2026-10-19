use super::*;
use std::io::Write;

fn control(name: &str, version: &str, arch: &str, description: &str) -> Control {
    crate::deb::parse_control_entry(&format!(
        "Package: {name}\nVersion: {version}\nArchitecture: {arch}\nDescription: {description}\n\n"
    ))
    .unwrap()
}

fn source(arch: &str, suite: &str) -> Source {
    Source {
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
        status_path: PathBuf::new(),
        files: Vec::new(),
    }
}

fn add(state: &mut AptState, name: &str, version: &str, arch: &str) {
    state.add_index(
        vec![control(name, version, arch, "test")],
        vec![source(arch, "stable")],
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
    state.add_index(
        vec![control("foo", "2", "amd64", "preview")],
        vec![source("amd64", "preview")],
    );

    let exact = state.resolve(vec!["foo=1".into()]);
    assert_eq!(exact.resolved[0].versions.len(), 1);
    assert_eq!(exact.resolved[0].versions[0].version(), Some("1"));

    let branch = state.resolve(vec!["foo/preview".into()]);
    assert_eq!(branch.resolved[0].versions.len(), 1);
    assert_eq!(branch.resolved[0].versions[0].version(), Some("2"));
}

#[test]
fn status_merges_into_packages_and_adds_missing_ones() {
    let mut state = state("amd64", &["amd64", "i386"]);
    add(&mut state, "foo", "1", "amd64");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("status");
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(
        b"Package: foo\nVersion: 1\nArchitecture: amd64\nStatus: install ok installed\n\n\
Package: vendor-tool\nVersion: 7\nArchitecture: amd64\nStatus: install ok installed\n\n\
Package: vendor-tool\nVersion: 7\nArchitecture: i386\nStatus: install ok installed\n\n\
Package: removed-tool\nVersion: 1\nArchitecture: amd64\nStatus: deinstall ok config-files\n\n",
    )
    .unwrap();
    state.status_path = path;
    state.merge_status();

    let foo = state.package("foo", &Arch::new("amd64")).unwrap();
    assert_eq!(foo.installed_version(), Some("1"));
    assert!(
        foo.versions()
            .iter()
            .any(|version| version.sources.iter().any(Source::is_dpkg_status))
    );

    let plain = state.resolve(vec!["vendor-tool".into()]);
    assert_eq!(plain.resolved[0].key.arch.as_str(), "amd64");
    assert!(state.is_installed(&plain.resolved[0].key));
    assert_eq!(
        state.resolve(vec!["vendor-tool:i386".into()]).resolved[0]
            .key
            .arch
            .as_str(),
        "i386"
    );
    assert_eq!(
        state.resolve(vec!["removed-tool".into()]).no_match,
        vec!["removed-tool"]
    );
}

#[test]
fn state_cache_roundtrips() {
    let mut state = state("amd64", &["amd64", "i386"]);
    add(&mut state, "foo", "1", "amd64");
    add(&mut state, "foo", "2", "i386");
    add(&mut state, "foo", "3", "all");

    let data = AptStateData {
        groups: state.groups.clone(),
        package_order: state.package_order.clone(),
        files: Vec::new(),
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("oma-aptdb.bincode");
    save_state_cache(path.to_str().unwrap(), &data).unwrap();

    let loaded = load_state_cache(path.to_str().unwrap()).unwrap();
    assert_eq!(loaded.groups.len(), 1);
    let packages = &loaded.groups["foo"];
    assert_eq!(packages.len(), 2);
    assert_eq!(
        loaded.package_order["foo"],
        vec![Arch::new("amd64"), Arch::new("i386")]
    );
    assert_eq!(packages[&Arch::new("amd64")].len(), 2);
}
