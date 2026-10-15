use super::*;

use std::collections::HashSet;
use std::str::FromStr;

use deb822_lossless::Deb822;

use crate::DpkgState;
use crate::apt_lists::PackageEntry;
use debian_control::relations::VersionConstraint;

fn base_entry() -> PackageEntry {
    PackageEntry {
        package: String::new(),
        version: None,
        architecture: Some("amd64".to_string()),
        description: None,
        description_md5: None,
        maintainer: None,
        installed_size: None,
        depends: None,
        pre_depends: None,
        recommends: None,
        suggests: None,
        breaks: None,
        conflicts: None,
        replaces: None,
        provides: None,
        section: None,
        priority: None,
        homepage: None,
        multi_arch: None,
        filename: None,
        size: None,
        sha256: None,
        essential: None,
        protected: None,
    }
}

fn entry(
    name: &str,
    version: &str,
    depends: Option<&str>,
    provides: Option<&str>,
    conflicts: Option<&str>,
    breaks: Option<&str>,
) -> PackageEntry {
    PackageEntry {
        package: name.to_string(),
        version: Some(version.to_string()),
        depends: depends.map(str::to_string),
        provides: provides.map(str::to_string),
        conflicts: conflicts.map(str::to_string),
        breaks: breaks.map(str::to_string),
        ..base_entry()
    }
}

fn db(entries: Vec<PackageEntry>) -> crate::AptDb {
    crate::AptDb::from_entries("", entries)
}

/// Collect the package names from a `Change` iterator (test helper).
fn mark_names<'a>(iter: impl Iterator<Item = &'a Change>) -> Vec<String> {
    iter.map(|c| c.package.clone()).collect()
}

/// Label a dpkg operation the way `apt --dry-run` prints it (test helper).
fn op_tag<'a>(op: &DpkgOp<'a>) -> (&'static str, &'a str) {
    match op {
        DpkgOp::Remove { package, .. } => ("Remv", *package),
        DpkgOp::Unpack { package, .. } => ("Inst", *package),
        DpkgOp::Configure { package, .. } => ("Conf", *package),
    }
}

/// Build a `DpkgState` from raw dpkg status content (test helper).
fn status_dpkg(content: &str) -> DpkgState {
    DpkgState::from_tree(Deb822::from_str(content).unwrap())
}

fn dpkg(installed: &[(&str, &str)]) -> DpkgState {
    let mut content = String::new();
    for (name, version) in installed {
        content.push_str(&format!(
            "Package: {name}\nVersion: {version}\nStatus: install ok installed\n\n"
        ));
    }
    status_dpkg(&content)
}

#[test]
fn test_transaction_install() {
    // liba 1.0 installed; app (new) needs liba >= 2.0 → upgrade liba.
    let index = db(vec![
        entry("app", "1.0", Some("liba (>= 2.0)"), None, None, None),
        entry("liba", "2.5", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("liba", "1.0")]);
    let txn = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(entry("app", "1.0", None, None, None, None), false)
        .resolve()
        .unwrap()
        .into_transaction();
    let app = txn.changes.iter().find(|c| c.package == "app").unwrap();
    assert_eq!(app.kind, ChangeKind::Install);
    assert_eq!(app.to_version.as_deref(), Some("1.0"));
    let liba = txn.changes.iter().find(|c| c.package == "liba").unwrap();
    assert_eq!(liba.kind, ChangeKind::Upgrade);
    assert_eq!(liba.from_version.as_deref(), Some("1.0"));
    assert_eq!(liba.to_version.as_deref(), Some("2.5"));
}

#[test]
fn test_change_carries_sizes() {
    // liba 1.0 installed (still present in the index); app and liba 2.5 are
    // the resolver's selections. Every change must carry its size impact.
    let mut liba_old = entry("liba", "1.0", None, None, None, None);
    liba_old.installed_size = Some(1000);
    liba_old.size = Some(500);
    let mut liba_new = entry("liba", "2.5", None, None, None, None);
    liba_new.installed_size = Some(2000);
    liba_new.size = Some(900);
    let mut app = entry("app", "1.0", Some("liba (>= 2.0)"), None, None, None);
    app.installed_size = Some(5000);
    app.size = Some(3000);

    let index = db(vec![app, liba_old, liba_new]);
    let dpkg = dpkg(&[("liba", "1.0")]);
    let marks = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(entry("app", "1.0", None, None, None, None), false)
        .resolve()
        .unwrap();

    let app = marks.changes.iter().find(|c| c.package == "app").unwrap();
    assert_eq!(app.kind, ChangeKind::Install);
    assert_eq!(app.old_size, None);
    assert_eq!(app.new_size, Some(5000));
    assert_eq!(app.download_size, Some(3000));

    let liba = marks.changes.iter().find(|c| c.package == "liba").unwrap();
    assert_eq!(liba.kind, ChangeKind::Upgrade);
    assert_eq!(liba.old_size, Some(1000));
    assert_eq!(liba.new_size, Some(2000));
    assert_eq!(liba.download_size, Some(900));
}

#[test]
fn test_remove_carries_size() {
    // A removal frees the installed size; no download or new size applies.
    let mut app = entry("app", "1.0", None, None, None, None);
    app.installed_size = Some(5000);
    let index = db(vec![app]);
    let dpkg = dpkg(&[("app", "1.0")]);
    let marks = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_remove(entry("app", "1.0", None, None, None, None))
        .resolve()
        .unwrap();

    let app = marks.changes.iter().find(|c| c.package == "app").unwrap();
    assert_eq!(app.kind, ChangeKind::Remove);
    assert_eq!(app.old_size, Some(5000));
    assert_eq!(app.new_size, None);
    assert_eq!(app.download_size, None);
}

#[test]
fn test_mark_install_pins_marked_version() {
    // Marking an entry with a version pins the root to that exact version:
    // the repo has app 1.0 and 2.0, but the marked entry says 1.0 → 1.0 wins.
    let index = db(vec![
        entry("app", "1.0", None, None, None, None),
        entry("app", "2.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[]);
    let changes = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(entry("app", "1.0", None, None, None, None), false)
        .resolve()
        .unwrap();
    let app = changes.changes.iter().find(|c| c.package == "app").unwrap();
    assert_eq!(app.kind, ChangeKind::Install);
    assert_eq!(app.to_version.as_deref(), Some("1.0"));
}

#[test]
fn test_mark_install_bare_entry_picks_newest() {
    // A marked entry without a version leaves the version to the resolver:
    // with app 1.0 and 2.0 in the repo, the newest (2.0) wins.
    let index = db(vec![
        entry("app", "1.0", None, None, None, None),
        entry("app", "2.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[]);
    let mut app = entry("app", "1.0", None, None, None, None);
    app.version = None;
    let changes = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(app, false)
        .resolve()
        .unwrap();
    let app = changes.changes.iter().find(|c| c.package == "app").unwrap();
    assert_eq!(app.kind, ChangeKind::Install);
    assert_eq!(app.to_version.as_deref(), Some("2.0"));
}

#[test]
fn test_resolve_consumes_marks() {
    // resolve() takes the pending marks, so the same planner can mark and
    // resolve again for a fresh transaction.
    let index = db(vec![
        entry("app", "1.0", None, None, None, None),
        entry("other", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[]);

    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    planner.mark_install(entry("app", "1.0", None, None, None, None), false);
    let first = planner.resolve().unwrap();
    assert!(first.changes.iter().any(|c| c.package == "app"));

    // Marks were consumed — resolving again without new marks is a no-op.
    let empty = planner.resolve().unwrap();
    assert!(empty.is_empty());

    // New marks start a fresh transaction.
    planner.mark_remove(entry("other", "1.0", None, None, None, None));
    let second = planner.resolve().unwrap();
    assert!(second.changes.iter().any(|c| c.package == "other"));
}

#[test]
fn test_upgrade_marks_outdated_packages() {
    // lib 1.0 installed, repo has 2.0 → upgrade() marks it; git is already
    // current → untouched. resolve() then yields the Upgrade change.
    let index = db(vec![
        entry("lib", "2.0", None, None, None, None),
        entry("git", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("lib", "1.0"), ("git", "1.0")]);
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    planner.upgrade(UpgradeMode::FullUpgrade);
    let changes = planner.resolve().unwrap();
    let lib = changes.changes.iter().find(|c| c.package == "lib").unwrap();
    assert_eq!(lib.kind, ChangeKind::Upgrade);
    assert_eq!(lib.from_version.as_deref(), Some("1.0"));
    assert_eq!(lib.to_version.as_deref(), Some("2.0"));
    assert!(!changes.changes.iter().any(|c| c.package == "git"));
}

#[test]
fn test_upgrade_with_no_outdated_is_noop() {
    // Everything already at the candidate version → upgrade() marks nothing.
    let index = db(vec![entry("lib", "1.0", None, None, None, None)]);
    let dpkg = dpkg(&[("lib", "1.0")]);
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    planner.upgrade(UpgradeMode::FullUpgrade);
    let changes = planner.resolve().unwrap();
    assert!(changes.is_empty());
}

#[test]
fn test_upgrade_safe_holds_back_conflict() {
    // app 2.0 conflicts with installed `old`. SafeUpgrade (`apt upgrade`)
    // never removes installed packages → app is held back; FullUpgrade may
    // remove `old` and upgrade app.
    let index = db(vec![
        entry("app", "2.0", None, None, Some("old"), None),
        entry("old", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("app", "1.0"), ("old", "1.0")]);

    // SafeUpgrade: hold back app (upgrading would remove `old`).
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    planner.upgrade(UpgradeMode::SafeUpgrade);
    assert!(planner.resolve().unwrap().is_empty());

    // FullUpgrade: allowed to remove `old` and upgrade app.
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    planner.upgrade(UpgradeMode::FullUpgrade);
    let changes = planner.resolve().unwrap();
    assert!(
        changes
            .changes
            .iter()
            .any(|c| c.package == "old" && c.kind == ChangeKind::Remove)
    );
    assert!(
        changes
            .changes
            .iter()
            .any(|c| c.package == "app" && c.kind == ChangeKind::Upgrade)
    );
}

#[test]
fn test_upgrade_minimal_holds_back_new_deps() {
    // app 2.0 Depends on libnew (not installed). MinimalUpgrade
    // (`apt-get upgrade`) never installs new packages → app is held back;
    // SafeUpgrade may install libnew and upgrade app.
    let index = db(vec![
        entry("app", "2.0", Some("libnew"), None, None, None),
        entry("libnew", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("app", "1.0")]);

    // MinimalUpgrade: held back — libnew is not installed.
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    planner.upgrade(UpgradeMode::MinimalUpgrade);
    assert!(planner.resolve().unwrap().is_empty());

    // SafeUpgrade: libnew may be installed.
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    planner.upgrade(UpgradeMode::SafeUpgrade);
    let changes = planner.resolve().unwrap();
    assert!(
        changes
            .changes
            .iter()
            .any(|c| c.package == "app" && c.kind == ChangeKind::Upgrade)
    );
    assert!(
        changes
            .changes
            .iter()
            .any(|c| c.package == "libnew" && c.kind == ChangeKind::Install)
    );
}

#[test]
fn test_upgrade_minimal_upgrades_when_deps_installed() {
    // app 2.0 Depends on lib (>= 2.0); both are installed at 1.0 and both
    // have 2.0 candidates. MinimalUpgrade can upgrade both — no new packages
    // and no removals are needed.
    let index = db(vec![
        entry("app", "2.0", Some("lib (>= 2.0)"), None, None, None),
        entry("lib", "2.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("app", "1.0"), ("lib", "1.0")]);
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    planner.upgrade(UpgradeMode::MinimalUpgrade);
    let changes = planner.resolve().unwrap();
    assert!(
        changes
            .changes
            .iter()
            .any(|c| c.package == "app" && c.kind == ChangeKind::Upgrade)
    );
    assert!(
        changes
            .changes
            .iter()
            .any(|c| c.package == "lib" && c.kind == ChangeKind::Upgrade)
    );
}

#[test]
fn test_marked_predicates() {
    // marked_install / marked_new_install / marked_reinstall / marked_remove
    // report the pending mark state before resolve.
    let index = db(vec![
        entry("lib", "2.0", None, None, None, None),
        entry("app", "1.0", None, None, None, None),
        entry("reapp", "1.0", None, None, None, None),
        entry("obsolete", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("lib", "1.0"), ("reapp", "1.0"), ("obsolete", "1.0")]);

    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    // Nothing marked yet.
    assert!(!planner.marked_install("lib"));
    assert!(!planner.marked_new_install("lib"));
    assert!(!planner.marked_reinstall("reapp"));
    assert!(!planner.marked_remove("obsolete"));

    planner.mark_install(entry("lib", "2.0", None, None, None, None), false);
    planner.mark_install(entry("app", "1.0", None, None, None, None), false);
    planner.mark_install(entry("reapp", "1.0", None, None, None, None), true);
    planner.mark_remove(entry("obsolete", "1.0", None, None, None, None));

    // Both lib and app are marked for install…
    assert!(planner.marked_install("lib"));
    assert!(planner.marked_install("app"));
    // …but only `app` (not installed) is a *new* install; `lib` is already
    // installed, so it is an upgrade, not a fresh install.
    assert!(!planner.marked_new_install("lib"));
    assert!(planner.marked_new_install("app"));
    // reapp is marked for reinstall; obsolete for removal.
    assert!(planner.marked_reinstall("reapp"));
    assert!(!planner.marked_reinstall("app"));
    assert!(planner.marked_remove("obsolete"));
    assert!(!planner.marked_remove("app"));

    // resolve consumes the marks: the predicates go quiet again.
    planner.resolve().unwrap();
    assert!(!planner.marked_install("app"));
    assert!(!planner.marked_new_install("app"));
    assert!(!planner.marked_reinstall("reapp"));
    assert!(!planner.marked_remove("obsolete"));
}

#[test]
fn test_clear_marked() {
    // clear_marked drops every pending mark without resolving, so the same
    // planner can start a fresh transaction.
    let index = db(vec![
        entry("app", "1.0", None, None, None, None),
        entry("obsolete", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("obsolete", "1.0")]);

    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    planner.mark_install(entry("app", "1.0", None, None, None, None), false);
    planner.mark_remove(entry("obsolete", "1.0", None, None, None, None));
    assert!(planner.marked_install("app"));
    assert!(planner.marked_remove("obsolete"));

    planner.clear_marked();
    assert!(!planner.marked_install("app"));
    assert!(!planner.marked_new_install("app"));
    assert!(!planner.marked_remove("obsolete"));

    // Nothing left to resolve — a fresh transaction can be marked.
    assert!(planner.resolve().unwrap().is_empty());
    planner.mark_install(entry("app", "1.0", None, None, None, None), false);
    assert!(planner.marked_install("app"));
    planner.resolve().unwrap();
    assert!(!planner.marked_install("app"));
}

#[test]
fn test_transaction_install_noop_when_current() {
    // Everything already installed at the selected version → no changes.
    let index = db(vec![
        entry("app", "1.0", Some("liba"), None, None, None),
        entry("liba", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("app", "1.0"), ("liba", "1.0")]);
    let txn = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(entry("app", "1.0", None, None, None, None), false)
        .resolve()
        .unwrap()
        .into_transaction();
    assert!(txn.is_empty());
}

#[test]
fn test_transaction_reinstall_broken() {
    // app is installed at 1.0 but dpkg flags it `reinstreq` → reinstalling the
    // same version produces a `Reinstall` change instead of a no-op.
    let index = db(vec![entry("app", "1.0", None, None, None, None)]);
    // dpkg flags app `reinstreq` → it needs a reinstall.
    let dpkg =
        status_dpkg("Package: app\nVersion: 1.0\nStatus: install reinstreq half-installed\n\n");
    let txn = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(entry("app", "1.0", None, None, None, None), false)
        .resolve()
        .unwrap()
        .into_transaction();
    let app = txn.changes.iter().find(|c| c.package == "app").unwrap();
    assert_eq!(app.kind, ChangeKind::Reinstall);
    assert_eq!(app.from_version.as_deref(), Some("1.0"));
    assert_eq!(app.to_version.as_deref(), Some("1.0"));
}

#[test]
fn test_mark_install_reinstall_same_version() {
    // app installed at 1.0 (repo has only 1.0): mark_install with
    // reinstall=false leaves it untouched; with reinstall=true it produces a
    // `Reinstall` change. A not-installed package marked reinstall is still a
    // fresh `Install`.
    let index = db(vec![
        entry("app", "1.0", None, None, None, None),
        entry("fresh", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("app", "1.0")]);

    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    planner.mark_install(entry("app", "1.0", None, None, None, None), false);
    assert!(planner.resolve().unwrap().is_empty());

    planner.mark_install(entry("app", "1.0", None, None, None, None), true);
    planner.mark_install(entry("fresh", "1.0", None, None, None, None), true);
    assert!(planner.marked_reinstall("app"));
    let txn = planner.resolve().unwrap().into_transaction();
    let app = txn.changes.iter().find(|c| c.package == "app").unwrap();
    assert_eq!(app.kind, ChangeKind::Reinstall);
    assert_eq!(app.from_version.as_deref(), Some("1.0"));
    assert_eq!(app.to_version.as_deref(), Some("1.0"));
    let fresh = txn.changes.iter().find(|c| c.package == "fresh").unwrap();
    assert_eq!(fresh.kind, ChangeKind::Install);
}

#[test]
fn test_transaction_auto_installed_flag() {
    // app is an explicit root (manually installed); its dependency liba is
    // pulled in by the resolver — only liba carries the auto-installed flag,
    // which the executor turns into `Auto-Installed: 1` records.
    let index = db(vec![
        entry("app", "1.0", Some("liba"), None, None, None),
        entry("liba", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[]);
    let txn = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(entry("app", "1.0", None, None, None, None), false)
        .resolve()
        .unwrap()
        .into_transaction();
    let app = txn.changes.iter().find(|c| c.package == "app").unwrap();
    assert!(!app.auto_installed);
    assert!(!app.is_auto_installed());
    let liba = txn.changes.iter().find(|c| c.package == "liba").unwrap();
    assert!(liba.auto_installed);
    assert!(liba.is_auto_installed());
    // into_transaction orders install-side in dependency order (liba first).
    assert_eq!(txn.auto_installed_names().collect::<Vec<_>>(), vec!["liba"]);
}

#[test]
fn test_fix_broken() {
    // app is installed at 1.0 but its hard dependency liba is not installed →
    // the installed version is broken. fix_broken marks app for install, and
    // resolve installs the missing dependency (auto-installed).
    let index = db(vec![
        entry("app", "1.0", Some("liba"), None, None, None),
        entry("liba", "1.0", None, None, None, None),
    ]);
    let dpkg_state = dpkg(&[("app", "1.0")]);
    let mut planner = TransactionPlanner::new(&index, &dpkg_state, ResolveOptions::default());
    let broken = planner.fix_broken().unwrap();
    assert_eq!(broken, vec!["app"]);
    let txn = planner.resolve().unwrap().into_transaction();
    // app itself is already present at the selected version and now healthy —
    // only the missing dependency is added.
    assert!(txn.changes.iter().all(|c| c.package != "app"));
    let liba = txn.changes.iter().find(|c| c.package == "liba").unwrap();
    assert_eq!(liba.kind, ChangeKind::Install);
    assert!(liba.auto_installed);

    // A package whose hard deps are met is not reported broken.
    let index_ok = db(vec![
        entry("ok", "1.0", Some("liba"), None, None, None),
        entry("liba", "1.0", None, None, None, None),
    ]);
    let dpkg_state_ok = dpkg(&[("ok", "1.0"), ("liba", "1.0")]);
    let mut planner_ok =
        TransactionPlanner::new(&index_ok, &dpkg_state_ok, ResolveOptions::default());
    assert!(planner_ok.fix_broken().unwrap().is_empty());
}

#[test]
fn test_fix_broken_satisfied_by_provider() {
    // app depends on the virtual `mail-transport-agent`, which the installed
    // `sendmail` provides → the installed version's deps are met, not broken.
    let index = db(vec![
        entry("app", "1.0", Some("mail-transport-agent"), None, None, None),
        entry(
            "sendmail",
            "1.0",
            None,
            Some("mail-transport-agent"),
            None,
            None,
        ),
    ]);
    let dpkg = dpkg(&[("app", "1.0"), ("sendmail", "1.0")]);
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    assert!(planner.fix_broken().unwrap().is_empty());
}

#[test]
fn test_fix_broken_upgrades_to_repair() {
    // app is installed at 1.0 but its dependency liba (>= 2.0) is unmet
    // (liba is not installed). fix_broken marks app (no pinned version), so
    // the resolver keeps the installed app and installs liba 2.0.
    let index = db(vec![
        entry("app", "1.0", Some("liba (>= 2.0)"), None, None, None),
        entry("liba", "2.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("app", "1.0")]);
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    assert_eq!(planner.fix_broken().unwrap(), vec!["app"]);
    let txn = planner.resolve().unwrap().into_transaction();
    let liba = txn.changes.iter().find(|c| c.package == "liba").unwrap();
    assert_eq!(liba.kind, ChangeKind::Install);
    assert_eq!(liba.to_version.as_deref(), Some("2.0"));
    // app itself stays at its installed version — only what's missing is added.
    assert!(txn.changes.iter().all(|c| c.package != "app"));
}

#[test]
fn test_autoremove_removes_unused_auto() {
    // app is manually installed and depends on liba (auto) → liba is kept;
    // libb (auto) is depended on by nothing → autoremoved.
    let index = db(vec![
        entry("app", "1.0", Some("liba"), None, None, None),
        entry("liba", "1.0", None, None, None, None),
        entry("libb", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("app", "1.0"), ("liba", "1.0"), ("libb", "1.0")]);
    let auto: HashSet<String> = ["liba", "libb"].into_iter().map(str::to_string).collect();

    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    assert_eq!(planner.autoremove(&auto, &[]), vec!["libb"]);
    let txn = planner.resolve().unwrap().into_transaction();
    assert_eq!(mark_names(txn.removals()), vec!["libb"]);
}

#[test]
fn test_autoremove_keeps_dependency_chain() {
    // app (manual) → liba (auto) → libb (auto): the whole chain is kept.
    let index = db(vec![
        entry("app", "1.0", Some("liba"), None, None, None),
        entry("liba", "1.0", Some("libb"), None, None, None),
        entry("libb", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("app", "1.0"), ("liba", "1.0"), ("libb", "1.0")]);
    let auto: HashSet<String> = ["liba", "libb"].into_iter().map(str::to_string).collect();

    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    assert!(planner.autoremove(&auto, &[]).is_empty());
    assert!(planner.resolve().unwrap().is_empty());
}

#[test]
fn test_autoremove_keeps_virtual_provider() {
    // app (manual) depends on the virtual mail-transport-agent; auto
    // sendmail provides it → kept. auto obsolete is not depended on →
    // removed.
    let index = db(vec![
        entry("app", "1.0", Some("mail-transport-agent"), None, None, None),
        entry(
            "sendmail",
            "1.0",
            None,
            Some("mail-transport-agent"),
            None,
            None,
        ),
        entry("obsolete", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("app", "1.0"), ("sendmail", "1.0"), ("obsolete", "1.0")]);
    let auto: HashSet<String> = ["sendmail", "obsolete"]
        .into_iter()
        .map(str::to_string)
        .collect();

    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    assert_eq!(planner.autoremove(&auto, &[]), vec!["obsolete"]);
    let txn = planner.resolve().unwrap().into_transaction();
    assert_eq!(mark_names(txn.removals()), vec!["obsolete"]);
}

#[test]
fn test_autoremove_honors_never_auto_remove() {
    // Packages matching APT::NeverAutoRemove patterns are roots — never
    // swept even when auto and unused (like the 01autoremove kernel /
    // firmware entries).
    let index = db(vec![
        entry("linux-image-amd64", "1.0", None, None, None, None),
        entry("obsolete", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("linux-image-amd64", "1.0"), ("obsolete", "1.0")]);
    let auto: HashSet<String> = ["linux-image-amd64", "obsolete"]
        .into_iter()
        .map(str::to_string)
        .collect();
    let never = ["^linux-image-[a-z0-9]*$".to_string()];

    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    assert_eq!(planner.autoremove(&auto, &never), vec!["obsolete"]);
    let txn = planner.resolve().unwrap().into_transaction();
    assert_eq!(mark_names(txn.removals()), vec!["obsolete"]);
}

#[test]
fn test_autoremove_skips_essential() {
    // An essential auto package is never autoremoved even if unused.
    let index = db(vec![entry("core", "1.0", None, None, None, None)]);
    // `Essential: yes` → the package is never autoremoved.
    let dpkg = status_dpkg(
        "Package: core\nVersion: 1.0\nStatus: install ok installed\nEssential: yes\n\n",
    );
    let auto: HashSet<String> = ["core"].into_iter().map(str::to_string).collect();
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    assert!(planner.autoremove(&auto, &[]).is_empty());
}

#[test]
fn test_autoremove_skips_held() {
    // A held auto package is a root — never autoremoved even if unused (apt
    // treats `Status: hold` like essential for non-user operations).
    let index = db(vec![
        entry("libheld", "1.0", None, None, None, None),
        entry("obsolete", "1.0", None, None, None, None),
    ]);
    // libheld is `Status: hold`, so it is a root even though unused.
    let dpkg = status_dpkg(
        "Package: libheld\nVersion: 1.0\nStatus: hold ok installed\n\n\
         Package: obsolete\nVersion: 1.0\nStatus: install ok installed\n\n",
    );
    let auto: HashSet<String> = ["libheld", "obsolete"]
        .into_iter()
        .map(str::to_string)
        .collect();
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    assert_eq!(planner.autoremove(&auto, &[]), vec!["obsolete"]);
    let txn = planner.resolve().unwrap().into_transaction();
    assert_eq!(mark_names(txn.removals()), vec!["obsolete"]);
}

#[test]
fn test_mark_held_protects_from_autoremove() {
    // mark_held flips the in-memory held state — a held auto package is
    // then a root, never swept (no `Status: hold` in dpkg needed).
    let index = db(vec![
        entry("libheld", "1.0", None, None, None, None),
        entry("obsolete", "1.0", None, None, None, None),
    ]);
    let mut dpkg = dpkg(&[("libheld", "1.0"), ("obsolete", "1.0")]);
    dpkg.mark_held("libheld").unwrap();
    assert!(dpkg.is_held("libheld"));
    let auto: HashSet<String> = ["libheld", "obsolete"]
        .into_iter()
        .map(str::to_string)
        .collect();
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    assert_eq!(planner.autoremove(&auto, &[]), vec!["obsolete"]);
}

#[test]
fn test_mark_held_refuses_not_installed() {
    // Like apt-mark: you cannot hold a package that is not installed.
    let mut dpkg = dpkg(&[("app", "1.0")]);
    let err = dpkg.mark_held("ghost").unwrap_err();
    assert!(matches!(err, crate::dpkg::DpkgError::NotInstalled(_)));
    assert!(!dpkg.is_held("ghost"));
}

#[test]
fn test_mark_held_already_held_is_noop() {
    // `Status: hold` already → mark_held is a no-op (no error, no rewrite).
    let mut dpkg = status_dpkg("Package: libheld\nVersion: 1.0\nStatus: hold ok installed\n\n");
    dpkg.mark_held("libheld").unwrap();
    assert!(dpkg.is_held("libheld"));
}

#[test]
fn test_mark_unheld_not_held_is_noop() {
    let mut dpkg = dpkg(&[("app", "1.0")]);
    dpkg.mark_unheld("app").unwrap();
    assert!(!dpkg.is_held("app"));
}

#[test]
fn test_mark_unheld_refuses_not_installed() {
    let mut dpkg = dpkg(&[("app", "1.0")]);
    let err = dpkg.mark_unheld("ghost").unwrap_err();
    assert!(matches!(err, crate::dpkg::DpkgError::NotInstalled(_)));
}

#[test]
fn test_held_to_file_round_trip() {
    // mark_held + to_file writes `Status: hold` into the dpkg status file;
    // from_file reads it back, so a hold survives a restart. Untouched
    // packages are preserved (lossless write).
    let dir = std::env::temp_dir().join("oma-held-to-file-test");
    std::fs::create_dir_all(&dir).ok();
    let status = dir.join("status");
    std::fs::write(
        &status,
        "\
Package: app
Version: 1.0
Status: install ok installed

Package: lib
Version: 1.0
Status: install ok installed

",
    )
    .unwrap();

    // Nothing is held yet.
    let mut dpkg = DpkgState::from_file(&status).unwrap();
    assert!(!dpkg.is_held("app"));
    assert!(!dpkg.is_held("lib"));

    // Mark and persist — app becomes `Status: hold` in the loaded tree, and
    // to_file writes it back.
    dpkg.mark_held("app").unwrap();
    assert!(dpkg.is_held("app"));
    dpkg.to_file(&status).unwrap();

    // A fresh load picks the hold back up — it survived the "restart".
    let reloaded = DpkgState::from_file(&status).unwrap();
    assert!(reloaded.is_held("app"));
    assert!(!reloaded.is_held("lib"));

    // The untouched package is preserved; app now carries the hold.
    let content = std::fs::read_to_string(&status).unwrap();
    assert!(content.contains("Status: hold ok installed"));
    assert!(content.contains("Package: lib"));
    assert!(content.contains("Status: install ok installed"));

    // mark_unheld removes the hold and persists it the same way.
    let mut unheld = DpkgState::from_file(&status).unwrap();
    unheld.mark_unheld("app").unwrap();
    assert!(!unheld.is_held("app"));
    unheld.to_file(&status).unwrap();
    let reloaded = DpkgState::from_file(&status).unwrap();
    assert!(!reloaded.is_held("app"));

    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn test_install_refuses_to_remove_held_conflict() {
    // libheld (hold) conflicts with the plan → resolve refuses instead of
    // removing the held package (apt's IsModeChangeOk: non-user removal of a
    // held package is blocked).
    let index = db(vec![
        entry("app", "1.0", None, None, Some("libheld"), None),
        entry("libheld", "1.0", None, None, None, None),
    ]);
    // libheld is `Status: hold`, so the plan cannot remove it.
    let dpkg = status_dpkg("Package: libheld\nVersion: 1.0\nStatus: hold ok installed\n\n");
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    planner.mark_install(entry("app", "1.0", None, None, None, None), false);
    assert!(matches!(planner.resolve(), Err(ResolveError::Held(_))));
}

#[test]
fn test_remove_held_explicit_is_allowed() {
    // `mark_remove` on a held package is a user request — like apt's
    // FromUser, it bypasses the hold protection.
    let index = db(vec![entry("heldpkg", "1.0", None, None, None, None)]);
    // Explicit `mark_remove` of a held package is a user request.
    let dpkg = status_dpkg("Package: heldpkg\nVersion: 1.0\nStatus: hold ok installed\n\n");
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    planner.mark_remove(entry("heldpkg", "1.0", None, None, None, None));
    let txn = planner.resolve().unwrap().into_transaction();
    assert_eq!(mark_names(txn.removals()), vec!["heldpkg"]);
}

#[test]
fn test_remove_refuses_held_reverse_dependent() {
    // Removing `lib` would pull in held `app` (its reverse hard-dependent);
    // the closure refuses to remove the held package, like apt's
    // IsModeChangeOk with FromUser=false.
    let index = db(vec![
        entry("app", "1.0", Some("lib"), None, None, None),
        entry("lib", "1.0", None, None, None, None),
    ]);
    // Removing `lib` would pull in held `app` (its reverse hard-dependent).
    let dpkg = status_dpkg(
        "Package: app\nVersion: 1.0\nStatus: hold ok installed\n\n\
         Package: lib\nVersion: 1.0\nStatus: install ok installed\n\n",
    );
    let mut planner = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default());
    planner.mark_remove(entry("lib", "1.0", None, None, None, None));
    assert!(matches!(planner.resolve(), Err(ResolveError::Held(_))));
}

#[test]
fn test_change_set_orders_removals_first() {
    // Mark phase produces unordered-ish changes; into_transaction must order
    // removals before installs (dpkg-safe: old conflicting packages are gone
    // before new ones are unpacked).
    let marks = ChangeSet {
        changes: vec![
            Change {
                kind: ChangeKind::Install,
                package: "app".into(),
                from_version: None,
                to_version: Some("1.0".into()),
                old_size: None,
                new_size: None,
                download_size: None,
                depends_on: Vec::new(),
                auto_installed: false,
            },
            Change {
                kind: ChangeKind::Remove,
                package: "obsolete".into(),
                from_version: Some("1.0".into()),
                to_version: None,
                old_size: None,
                new_size: None,
                download_size: None,
                depends_on: Vec::new(),
                auto_installed: false,
            },
        ],
    };
    let txn = marks.into_transaction();
    let kinds: Vec<ChangeKind> = txn.changes.iter().map(|c| c.kind).collect();
    assert_eq!(kinds, vec![ChangeKind::Remove, ChangeKind::Install]);
}

#[test]
fn test_change_set_per_kind_accessors() {
    // Every ChangeKind gets its own accessor on ChangeSet.
    let marks = ChangeSet {
        changes: vec![
            Change {
                kind: ChangeKind::Install,
                package: "a".into(),
                from_version: None,
                to_version: Some("1.0".into()),
                old_size: None,
                new_size: None,
                download_size: None,
                depends_on: Vec::new(),
                auto_installed: false,
            },
            Change {
                kind: ChangeKind::Upgrade,
                package: "b".into(),
                from_version: Some("1.0".into()),
                to_version: Some("2.0".into()),
                old_size: None,
                new_size: None,
                download_size: None,
                depends_on: Vec::new(),
                auto_installed: false,
            },
            Change {
                kind: ChangeKind::Downgrade,
                package: "c".into(),
                from_version: Some("2.0".into()),
                to_version: Some("1.0".into()),
                old_size: None,
                new_size: None,
                download_size: None,
                depends_on: Vec::new(),
                auto_installed: false,
            },
            Change {
                kind: ChangeKind::Reinstall,
                package: "d".into(),
                from_version: Some("1.0".into()),
                to_version: Some("1.0".into()),
                old_size: None,
                new_size: None,
                download_size: None,
                depends_on: Vec::new(),
                auto_installed: false,
            },
            Change {
                kind: ChangeKind::Remove,
                package: "e".into(),
                from_version: Some("1.0".into()),
                to_version: None,
                old_size: None,
                new_size: None,
                download_size: None,
                depends_on: Vec::new(),
                auto_installed: false,
            },
        ],
    };

    assert_eq!(mark_names(marks.installs()), vec!["a"]);
    assert_eq!(mark_names(marks.upgrades()), vec!["b"]);
    assert_eq!(mark_names(marks.downgrades()), vec!["c"]);
    assert_eq!(mark_names(marks.reinstalls()), vec!["d"]);
    assert_eq!(mark_names(marks.removals()), vec!["e"]);
    // of_kind is the general filter.
    assert_eq!(mark_names(marks.of_kind(ChangeKind::Downgrade)), vec!["c"]);

    // into_transaction keeps every kind: removals first, then the four
    // install-side kinds in dependency order, and Transaction exposes the
    // same per-kind accessors.
    let txn = marks.into_transaction();
    let kinds: Vec<ChangeKind> = txn.changes.iter().map(|c| c.kind).collect();
    assert_eq!(
        kinds,
        vec![
            ChangeKind::Remove,
            ChangeKind::Install,
            ChangeKind::Upgrade,
            ChangeKind::Downgrade,
            ChangeKind::Reinstall,
        ]
    );
    assert_eq!(mark_names(txn.installs()), vec!["a"]);
    assert_eq!(mark_names(txn.upgrades()), vec!["b"]);
    assert_eq!(mark_names(txn.downgrades()), vec!["c"]);
    assert_eq!(mark_names(txn.reinstalls()), vec!["d"]);
    assert_eq!(mark_names(txn.removals()), vec!["e"]);
}

#[test]
fn test_dpkg_plan_matches_apt_dry_run_order() {
    // The dpkg plan is a *separate* concern from the state changes: it lists
    // what dpkg does with each package, in the order apt --dry-run prints
    // (Remv, then Inst, then Conf). Each install-side change appears twice.
    let marks = ChangeSet {
        changes: vec![
            Change {
                kind: ChangeKind::Install,
                package: "app".into(),
                from_version: None,
                to_version: Some("1.0".into()),
                old_size: None,
                new_size: None,
                download_size: None,
                depends_on: Vec::new(),
                auto_installed: false,
            },
            Change {
                kind: ChangeKind::Upgrade,
                package: "liba".into(),
                from_version: Some("0.5".into()),
                to_version: Some("1.0".into()),
                old_size: None,
                new_size: None,
                download_size: None,
                depends_on: Vec::new(),
                auto_installed: false,
            },
            Change {
                kind: ChangeKind::Remove,
                package: "obsolete".into(),
                from_version: Some("1.0".into()),
                to_version: None,
                old_size: None,
                new_size: None,
                download_size: None,
                depends_on: Vec::new(),
                auto_installed: false,
            },
        ],
    };
    let txn = marks.into_transaction();
    let plan = txn.to_dpkg_plan();
    let ops: Vec<(&str, &str)> = plan.ops().iter().map(op_tag).collect();
    assert_eq!(
        ops,
        vec![
            ("Remv", "obsolete"),
            ("Inst", "app"),
            ("Inst", "liba"),
            ("Conf", "app"),
            ("Conf", "liba"),
        ]
    );
    // Versions are carried per operation: remove reports the installed one,
    // unpack/configure report the version being installed.
    assert_eq!(
        plan.ops()[0],
        DpkgOp::Remove {
            package: "obsolete",
            version: Some("1.0"),
        }
    );
}

#[test]
fn test_dpkg_plan_orders_install_side_in_dependency_order() {
    // app → libb → liba chain. mark_install leaves marks unordered (apt's
    // MarkInstall does not sort); into_transaction computes the topological
    // dpkg order (liba, libb, app) in the order phase, like pkgOrderList.
    let index = db(vec![
        entry("app", "1.0", Some("libb (>= 2.0)"), None, None, None),
        entry("libb", "2.0", Some("liba"), None, None, None),
        entry("liba", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[]);
    let marks = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(entry("app", "1.0", None, None, None, None), false)
        .resolve()
        .unwrap();
    let txn = marks.into_transaction();
    let plan = txn.to_dpkg_plan();
    let inst: Vec<&str> = plan
        .unpacks()
        .map(|op| match op {
            DpkgOp::Unpack { package, .. } => *package,
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(inst, vec!["liba", "libb", "app"]);
}

#[test]
fn test_transaction_install_removes_conflicting() {
    // app conflicts with old installed package `obsolete` → removed.
    let index = db(vec![
        entry("app", "1.0", None, None, Some("obsolete"), None),
        entry("obsolete", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("obsolete", "1.0")]);
    let txn = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(entry("app", "1.0", None, None, None, None), false)
        .resolve()
        .unwrap()
        .into_transaction();
    let obsolete = txn
        .changes
        .iter()
        .find(|c| c.package == "obsolete")
        .unwrap();
    assert_eq!(obsolete.kind, ChangeKind::Remove);
}

#[test]
fn test_transaction_removes_installed_conflicting_plan() {
    // Direction 2: installed `oldapp` Conflicts with the plan's `app` → the
    // installed package is removed (checked against its *installed* version's
    // entry, what is actually active on the system).
    let index = db(vec![
        entry("app", "1.0", None, None, None, None),
        entry("oldapp", "1.0", None, None, Some("app"), None),
    ]);
    let dpkg = dpkg(&[("oldapp", "1.0")]);
    let txn = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(entry("app", "1.0", None, None, None, None), false)
        .resolve()
        .unwrap()
        .into_transaction();
    let oldapp = txn.changes.iter().find(|c| c.package == "oldapp").unwrap();
    assert_eq!(oldapp.kind, ChangeKind::Remove);
}

#[test]
fn test_transaction_refuses_removing_essential() {
    // app conflicts with installed essential package `obsolete` → the mark
    // fails, like apt refusing to remove a protected package.
    let index = db(vec![
        entry("app", "1.0", None, None, Some("obsolete"), None),
        entry("obsolete", "1.0", None, None, None, None),
    ]);
    // `Essential: yes` → the plan cannot remove it.
    let dpkg = status_dpkg(
        "Package: obsolete\nVersion: 1.0\nStatus: install ok installed\nEssential: yes\n\n",
    );
    let err = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(entry("app", "1.0", None, None, None, None), false)
        .resolve()
        .unwrap_err();
    assert!(matches!(err, ResolveError::Essential(_)));
}

#[test]
fn test_transaction_refuses_removing_protected() {
    // app conflicts with installed `Protected: yes` package `obsolete` → the
    // mark fails too (dpkg 1.19+ Protected level, apt's `Flag::Important`).
    let index = db(vec![
        entry("app", "1.0", None, None, Some("obsolete"), None),
        entry("obsolete", "1.0", None, None, None, None),
    ]);
    // `Protected: yes` → the plan cannot remove it either.
    let dpkg = status_dpkg(
        "Package: obsolete\nVersion: 1.0\nStatus: install ok installed\nProtected: yes\n\n",
    );
    let err = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(entry("app", "1.0", None, None, None, None), false)
        .resolve()
        .unwrap_err();
    assert!(matches!(err, ResolveError::Protected(_)));
}

#[test]
fn test_transaction_removes_reverse_deps_of_conflict() {
    // app conflicts with installed `obsolete`; installed `dep` hard-depends
    // on obsolete → both are removed, like apt's MarkDelete marking
    // dependents for removal too.
    let index = db(vec![
        entry("app", "1.0", None, None, Some("obsolete"), None),
        entry("obsolete", "1.0", None, None, None, None),
        entry("dep", "1.0", Some("obsolete"), None, None, None),
    ]);
    let dpkg = dpkg(&[("obsolete", "1.0"), ("dep", "1.0")]);
    let txn = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(entry("app", "1.0", None, None, None, None), false)
        .resolve()
        .unwrap()
        .into_transaction();
    let removed: Vec<&str> = txn
        .changes
        .iter()
        .filter(|c| c.kind == ChangeKind::Remove)
        .map(|c| c.package.as_str())
        .collect();
    assert!(removed.contains(&"obsolete"));
    assert!(removed.contains(&"dep"));
}

#[test]
fn test_transaction_refuses_removing_essential_reverse_dep() {
    // app conflicts with installed `obsolete`; essential `dep` depends on
    // obsolete → removing obsolete would force removing an essential package,
    // so the mark fails.
    let index = db(vec![
        entry("app", "1.0", None, None, Some("obsolete"), None),
        entry("obsolete", "1.0", None, None, None, None),
        entry("dep", "1.0", Some("obsolete"), None, None, None),
    ]);
    // Essential `dep` depends on obsolete → removing obsolete would force
    // removing an essential package, so the mark fails.
    let dpkg = status_dpkg(
        "Package: obsolete\nVersion: 1.0\nStatus: install ok installed\n\n\
         Package: dep\nVersion: 1.0\nStatus: install ok installed\nEssential: yes\n\n",
    );
    let err = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_install(entry("app", "1.0", None, None, None, None), false)
        .resolve()
        .unwrap_err();
    assert!(matches!(err, ResolveError::Essential(_)));
}

#[test]
fn test_transaction_remove_reverse_deps() {
    // app depends on liba; removing liba must also remove app.
    let index = db(vec![
        entry("app", "1.0", Some("liba"), None, None, None),
        entry("liba", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("app", "1.0"), ("liba", "1.0")]);
    let txn = TransactionPlanner::new(&index, &dpkg, ResolveOptions::default())
        .mark_remove(entry("liba", "1.0", None, None, None, None))
        .resolve()
        .unwrap()
        .into_transaction();
    let names: Vec<&str> = txn.changes.iter().map(|c| c.package.as_str()).collect();
    assert!(names.contains(&"liba"));
    assert!(names.contains(&"app"));
    assert!(txn.changes.iter().all(|c| c.kind == ChangeKind::Remove));
}

#[test]
fn test_solve_simple_dep() {
    let index = db(vec![
        entry("app", "1.0", Some("libc6 (>= 2.0)"), None, None, None),
        entry("libc6", "2.5", None, None, None, None),
    ]);
    let sol = solve_packages(&index, &["app"]).unwrap();
    assert!(sol.iter().any(|(n, _)| n == "app"));
    assert!(sol.iter().any(|(n, v)| n == "libc6" && v == "2.5"));
}

#[test]
fn test_solve_picks_newest_version() {
    let index = db(vec![
        entry("app", "1.0", Some("libc6"), None, None, None),
        entry("libc6", "2.0", None, None, None, None),
        entry("libc6", "2.5", None, None, None, None),
    ]);
    let sol = solve_packages(&index, &["app"]).unwrap();
    let libc6 = sol.iter().find(|(n, _)| n == "libc6").unwrap();
    assert_eq!(libc6.1, "2.5");
}

#[test]
fn test_solve_or_alternative() {
    let index = db(vec![
        entry("app", "1.0", Some("zsh | fish"), None, None, None),
        entry("fish", "3.6", None, None, None, None),
    ]);
    let sol = solve_packages(&index, &["app"]).unwrap();
    assert!(sol.iter().any(|(n, _)| n == "fish"));
}

#[test]
fn test_solve_provides_virtual() {
    let index = db(vec![
        entry("app", "1.0", Some("mail-transport-agent"), None, None, None),
        entry(
            "postfix",
            "3.7",
            None,
            Some("mail-transport-agent"),
            None,
            None,
        ),
    ]);
    let sol = solve_packages(&index, &["app"]).unwrap();
    assert!(sol.iter().any(|(n, _)| n == "postfix"));
}

#[test]
fn test_solve_conflicts_avoids_conflicting() {
    let index = db(vec![
        entry("app", "1.0", Some("a"), None, Some("b"), None),
        entry("a", "1.0", None, None, None, None),
        entry("b", "1.0", None, None, None, None),
        entry("b", "2.0", None, None, None, None),
    ]);
    // app conflicts with b (unversioned) → b must be excluded entirely.
    let sol = solve_packages(&index, &["app"]).unwrap();
    assert!(sol.iter().any(|(n, _)| n == "app"));
    assert!(sol.iter().any(|(n, _)| n == "a"));
    assert!(!sol.iter().any(|(n, _)| n == "b"));
}

#[test]
fn test_solve_conflicts_version_range() {
    let index = db(vec![
        entry("app", "1.0", Some("b"), None, Some("b (<< 2.0)"), None),
        entry("b", "1.0", None, None, None, None),
        entry("b", "2.5", None, None, None, None),
    ]);
    // app conflicts with b < 2.0 → solver must pick b 2.5, not 1.0.
    let sol = solve_packages(&index, &["app"]).unwrap();
    let b = sol.iter().find(|(n, _)| n == "b").unwrap();
    assert_eq!(b.1, "2.5");
}

#[test]
fn test_solve_self_conflict_no_panic() {
    // A package that conflicts with older versions of itself must not
    // trip resolvo's self-clause assertion (apt: a package never
    // conflicts with itself).
    let index = db(vec![
        entry("foo", "1.0", None, None, Some("foo (<< 2.0)"), None),
        entry("foo", "2.0", None, None, None, None),
    ]);
    let sol = solve_packages(&index, &["foo"]).unwrap();
    let foo = sol.iter().find(|(n, _)| n == "foo").unwrap();
    assert_eq!(foo.1, "2.0");
}

#[test]
fn test_solve_version_constraint() {
    let index = db(vec![
        entry("foo", "1.0", None, None, None, None),
        entry("foo", "2.0", None, None, None, None),
    ]);
    let sol = solve_requirements(
        &index,
        &[(
            "foo",
            AptVersionSet::Constraint(VersionConstraint::GreaterThanEqual, "2.0".to_string()),
        )],
    )
    .unwrap();
    let foo = sol.iter().find(|(n, _)| n == "foo").unwrap();
    assert_eq!(foo.1, "2.0");
}

#[test]
fn test_solve_per_version_dependencies() {
    // foo 1.0 depends on liba, foo 2.0 depends on libb. Solving foo@1.0
    // must read the *1.0* entry's deps, not the candidate's.
    let index = db(vec![
        entry("foo", "1.0", Some("liba"), None, None, None),
        entry("foo", "2.0", Some("libb"), None, None, None),
        entry("liba", "1.0", None, None, None, None),
        entry("libb", "1.0", None, None, None, None),
    ]);
    let sol = solve_requirements(
        &index,
        &[(
            "foo",
            AptVersionSet::Constraint(VersionConstraint::Equal, "1.0".to_string()),
        )],
    )
    .unwrap();
    assert!(sol.iter().any(|(n, _)| n == "liba"));
    assert!(!sol.iter().any(|(n, _)| n == "libb"));
}

#[test]
fn test_resolve_install_order() {
    // app → libb → liba (chain). Install order must be liba, libb, app.
    let index = db(vec![
        entry("app", "1.0", Some("libb (>= 2.0)"), None, None, None),
        entry("libb", "2.0", Some("liba"), None, None, None),
        entry("liba", "1.0", None, None, None, None),
    ]);
    let plan = resolve_install_order(&index, &["app"]).unwrap();
    let names: Vec<&str> = plan.iter().map(|item| item.name.as_str()).collect();
    let pos = |n: &str| names.iter().position(|x| *x == n).unwrap();
    assert!(pos("liba") < pos("libb"));
    assert!(pos("libb") < pos("app"));
    // app's plan entry lists libb as a dependency.
    let app = plan.iter().find(|item| item.name == "app").unwrap();
    assert_eq!(app.depends_on, vec!["libb".to_string()]);
}

#[test]
fn test_resolve_install_order_or_virtual() {
    // app depends on a virtual name provided by postfix.
    let index = db(vec![
        entry("app", "1.0", Some("mail-transport-agent"), None, None, None),
        entry(
            "postfix",
            "3.7",
            None,
            Some("mail-transport-agent"),
            None,
            None,
        ),
    ]);
    let plan = resolve_install_order(&index, &["app"]).unwrap();
    let names: Vec<&str> = plan.iter().map(|item| item.name.as_str()).collect();
    assert!(names.contains(&"postfix"));
    // app's dependency must be mapped to the concrete provider.
    let app = plan.iter().find(|item| item.name == "app").unwrap();
    assert_eq!(app.depends_on, vec!["postfix".to_string()]);
}

#[test]
fn test_install_recommends_option() {
    // app Depends: liba, Recommends: libb.
    let index = db(vec![
        PackageEntry {
            package: "app".to_string(),
            version: Some("1.0".to_string()),
            depends: Some("liba".to_string()),
            recommends: Some("libb".to_string()),
            ..base_entry()
        },
        entry("liba", "1.0", None, None, None, None),
        entry("libb", "1.0", None, None, None, None),
    ]);
    // Default (apt's install-recommends): libb is pulled in.
    let plan = resolve_install_order(&index, &["app"]).unwrap();
    assert!(plan.iter().any(|item| item.name == "libb"));
    // Recommends off: only the hard closure.
    let plan = resolve_install_order_with(
        &index,
        &["app"],
        ResolveOptions {
            install_recommends: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(plan.iter().any(|item| item.name == "liba"));
    assert!(!plan.iter().any(|item| item.name == "libb"));
}

#[test]
fn test_suggests_optional() {
    // app Suggests libsug: only pulled into the closure when install_suggests
    // is enabled (APT::Install-Suggests).
    let mut app = entry("app", "1.0", None, None, None, None);
    app.suggests = Some("libsug".to_string());
    let index = db(vec![app, entry("libsug", "1.0", None, None, None, None)]);

    // default (suggests off): app only.
    let sol = solve_packages(&index, &["app"]).unwrap();
    assert!(sol.iter().any(|(n, _)| n == "app"));
    assert!(!sol.iter().any(|(n, _)| n == "libsug"));

    // suggests on: libsug is required.
    let sol = solve_requirements_with(
        &index,
        &[("app", AptVersionSet::Any)],
        ResolveOptions {
            install_recommends: true,
            install_suggests: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(sol.iter().any(|(n, _)| n == "libsug"));
}

#[test]
fn test_prefer_installed_keeps_installed_version() {
    // lib 1.0 installed; repo also has 2.0. With prefer_installed (default,
    // apt semantics) the resolver keeps 1.0 — no spurious upgrade; without it
    // the newest candidate (2.0) wins.
    let index = db(vec![
        entry("app", "1.0", Some("lib"), None, None, None),
        entry("lib", "1.0", None, None, None, None),
        entry("lib", "2.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("lib", "1.0")]);

    // Prefer installed (default): keep lib 1.0.
    let plan = resolve_plan(
        &index,
        Some(&dpkg),
        &[("app", AptVersionSet::Any)],
        ResolveOptions::default(),
    )
    .unwrap();
    let lib = plan.iter().find(|item| item.name == "lib").unwrap();
    assert_eq!(lib.version, "1.0");

    // prefer_installed off: newest (2.0) wins.
    let plan = resolve_install_order_with(
        &index,
        &["app"],
        ResolveOptions {
            prefer_installed: false,
            ..Default::default()
        },
    )
    .unwrap();
    let lib = plan.iter().find(|item| item.name == "lib").unwrap();
    assert_eq!(lib.version, "2.0");
}

#[test]
fn test_prefer_installed_keeps_version_absent_from_repo() {
    // lib installed at 2.0, but the repo only carries 1.0 (like the real
    // libsigc++-3.0 case: installed 3.6.0, repo only 3.4.0). apt reads the
    // installed version from dpkg status and keeps it; with prefer_installed
    // the resolver must too, instead of downgrading to 1.0.
    let index = db(vec![
        entry("app", "1.0", Some("lib"), None, None, None),
        entry("lib", "1.0", None, None, None, None),
    ]);
    let dpkg = dpkg(&[("lib", "2.0")]);

    // Prefer installed (default): keep lib 2.0.
    let plan = resolve_plan(
        &index,
        Some(&dpkg),
        &[("app", AptVersionSet::Any)],
        ResolveOptions::default(),
    )
    .unwrap();
    let lib = plan.iter().find(|item| item.name == "lib").unwrap();
    assert_eq!(lib.version, "2.0");

    // prefer_installed off: only repo candidates → downgrade to 1.0.
    let plan = resolve_install_order_with(
        &index,
        &["app"],
        ResolveOptions {
            prefer_installed: false,
            ..Default::default()
        },
    )
    .unwrap();
    let lib = plan.iter().find(|item| item.name == "lib").unwrap();
    assert_eq!(lib.version, "1.0");
}

#[test]
fn test_resolve_options_from_apt_config() {
    let mut cfg = crate::AptConfig::new();
    cfg.set("APT::Install-Recommends", "false");
    cfg.set("APT::Install-Suggests", "true");
    let opts = ResolveOptions::from(&cfg);
    assert!(!opts.install_recommends);
    assert!(opts.install_suggests);

    // absent keys → apt defaults (recommends on, suggests off).
    let opts = ResolveOptions::from(&crate::AptConfig::new());
    assert!(opts.install_recommends);
    assert!(!opts.install_suggests);
}

#[test]
fn test_solve_unsolvable() {
    let index = db(vec![entry(
        "app",
        "1.0",
        Some("missing-pkg"),
        None,
        None,
        None,
    )]);
    assert!(solve_packages(&index, &["app"]).is_err());
}

/// EDSP-style arch-qualified plan: `app:amd64` depends on `bar:amd64` — the
/// dependency edge must point at the concrete arch-qualified entry, not the
/// bare name (which would miss the `name:arch` plan keys).
#[test]
fn test_resolve_plan_depends_on_arch_qualified() {
    let index = db(vec![
        entry("app:amd64", "1.0", Some("bar:amd64"), None, None, None),
        entry("bar:amd64", "1.0", None, None, None, None),
    ]);
    let plan = resolve_plan(
        &index,
        None,
        &[("app:amd64", AptVersionSet::Any)],
        ResolveOptions::default(),
    )
    .unwrap();
    let app = plan.iter().find(|item| item.name == "app:amd64").unwrap();
    assert!(
        app.depends_on.contains(&"bar:amd64".to_string()),
        "app edges must point at bar:amd64: {:?}",
        app.depends_on
    );
}

/// A `Multi-Arch: foreign` bare dependency qualifies to `bar:any`; the plan
/// selected `bar:i386` — the edge must resolve to it via the base name.
#[test]
fn test_resolve_plan_depends_on_any_matches_foreign_arch() {
    let index = db(vec![
        entry("app:amd64", "1.0", Some("bar:any"), None, None, None),
        entry("bar:i386", "1.0", None, None, None, None),
    ]);
    let plan = resolve_plan(
        &index,
        None,
        &[("app:amd64", AptVersionSet::Any)],
        ResolveOptions::default(),
    )
    .unwrap();
    let app = plan.iter().find(|item| item.name == "app:amd64").unwrap();
    assert!(
        app.depends_on.contains(&"bar:i386".to_string()),
        "app edges must resolve bar:any to the selected bar:i386: {:?}",
        app.depends_on
    );
}
