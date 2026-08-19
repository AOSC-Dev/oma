//! EDSP (External Dependency Solving Protocol) support.
//!
//! apt can delegate dependency resolution to an external solver binary: it
//! runs the binary with the request and the whole package universe on stdin
//! (deb822, see `doc/external-dependency-solver-protocol.md` in the apt
//! source) and reads the solution from stdout. This module implements that
//! protocol on top of the resolvo-based resolver:
//!
//! - [`parse_input`] reads the request stanza + package universe
//! - [`solve`] / [`solve_with`] run the resolver and map the result back to
//!   the `APT-ID`s apt expects
//! - [`write_solution`] / [`write_error`] format the answer
//!
//! The companion binary `oma-edsp` is the thin wrapper apt invokes: install
//! it (or a symlink) into `Dir::Bin::solvers` (default
//! `/usr/libexec/apt/solvers`) and select it with `APT::Solver=oma-edsp` or
//! `apt-get -o APT::Solver=oma-edsp …`.
//!
//! The EDSP protocol fields live only here, in [`EdspVersion`] — they are
//! transport details of the protocol, not package metadata, so
//! [`PackageEntry`](crate::PackageEntry) (the apt-list record type) stays
//! untouched. The solver consumes an [`AptDb`] built from the universe;
//! [`EdspIndex`] wraps that database and overrides candidate selection with
//! apt's `APT-Candidate` marker (so pins apt already applied are honoured),
//! letting the mark phase pin each install root to apt's candidate. The
//! installed state is synthesised from
//! the universe's `Installed: yes` / `Hold: yes` fields — the solver never
//! reads `/var/lib/dpkg/status`.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io::Write;

use crate::apt_provider::{ChangeKind, TransactionPlanner, UpgradeMode};
use crate::{AptDb, DpkgIndex, DpkgState, PackageEntry, PackageVersion, ResolveOptions};

/// The parsed EDSP request stanza — what apt wants done.
///
/// Borrows from the raw request text — parsing copies nothing.
#[derive(Debug, Clone)]
pub struct EdspRequest<'a> {
    /// Native architecture (`Architecture:` in the request).
    pub architecture: &'a str,
    /// Packages to install (arch-qualified names, `pkg:arch`).
    pub install: Vec<&'a str>,
    /// Packages to remove (arch-qualified names).
    pub remove: Vec<&'a str>,
    /// `Upgrade` / `Upgrade-All` / `Dist-Upgrade` was requested.
    pub upgrade_all: bool,
    /// `Forbid-New-Install: yes` — the resolver must not install new packages.
    pub forbid_new_install: bool,
    /// `Forbid-Remove: yes` — the resolver must not remove packages.
    pub forbid_remove: bool,
    /// `Autoremove: yes` — report the installed auto-installed packages that
    /// are no longer needed as `Autoremove:` stanzas.
    pub autoremove: bool,
}

/// One package version of the EDSP package universe — the protocol fields
/// (`APT-ID`, `Installed`, `Hold`, `APT-Candidate`, `APT-Automatic`) plus
/// the dependency fields the resolver reads.
///
/// All strings borrow from the raw universe text — parsing copies nothing.
#[derive(Debug, Clone)]
pub struct EdspVersion<'a> {
    pub package: &'a str,
    pub version: &'a str,
    /// The package's architecture. EDSP requires every universe version to
    /// carry `Architecture:`, so a missing field is a parse error, never
    /// silently treated as `all`.
    pub architecture: &'a str,
    /// The `APT-ID` apt uses to reference this version in the solution.
    pub apt_id: u64,
    /// `Installed: yes` — this version is the one currently installed.
    pub installed: bool,
    /// `Hold: yes` — the package is on hold.
    pub hold: bool,
    /// `Essential: yes`.
    pub essential: bool,
    /// `APT-Candidate: yes` — apt's policy selected this version.
    pub apt_candidate: bool,
    /// `APT-Automatic: yes` — installed automatically as a dependency.
    pub apt_automatic: bool,
    pub depends: Option<&'a str>,
    pub pre_depends: Option<&'a str>,
    pub recommends: Option<&'a str>,
    pub suggests: Option<&'a str>,
    pub breaks: Option<&'a str>,
    pub conflicts: Option<&'a str>,
    pub replaces: Option<&'a str>,
    pub provides: Option<&'a str>,
    /// `Multi-Arch:` — apt's EDSP output marks `foreign` packages only. A
    /// bare dependency on a `foreign` package may be satisfied by any
    /// architecture of it, while every other package resolves to the
    /// referrer's own architecture (or `Architecture: all`).
    pub multi_arch: Option<&'a str>,
}

impl<'a> EdspVersion<'a> {
    /// The arch-qualified package identity, `name:arch` — the multi-arch key
    /// the resolver uses. `foo:amd64` and `foo:i386` are distinct packages;
    /// `Architecture: all` becomes `name:all` (which satisfies any arch).
    fn fullname(&self) -> String {
        format!("{}:{}", self.package, self.architecture)
    }

    /// The dependency/display fields as a [`PackageEntry`] the solver can
    /// read. Bare dependency names are qualified for this version's
    /// architecture (`libbar` in `foo:amd64` means `libbar:amd64`, and an
    /// `Architecture: all` package also satisfies it), so the resolver never
    /// merges architectures or wrongly resolves a dependency across them.
    fn to_entry(&self, native_arch: &str, foreign: &HashSet<&str>) -> PackageEntry {
        let arch = self.architecture;
        // `Architecture: all` packages' own dependencies bind to the native
        // architecture — they can be installed for any arch.
        let effective = if arch == "all" { native_arch } else { arch };
        PackageEntry {
            package: self.fullname(),
            version: Some(self.version.to_string()),
            architecture: Some(self.architecture.to_string()),
            multi_arch: self.multi_arch.map(str::to_string),
            depends: self
                .depends
                .map(|f| qualify_dep_field(f, effective, foreign)),
            pre_depends: self
                .pre_depends
                .map(|f| qualify_dep_field(f, effective, foreign)),
            recommends: self
                .recommends
                .map(|f| qualify_dep_field(f, effective, foreign)),
            suggests: self
                .suggests
                .map(|f| qualify_dep_field(f, effective, foreign)),
            // Breaks/Conflicts/Replaces are arch-unconstrained: a bare
            // `Conflicts: dirmngr` conflicts with *every* architecture of
            // dirmngr (`dirmngr:any`), not just `arch | all`.
            breaks: self.breaks.map(qualify_conflicts_field),
            conflicts: self.conflicts.map(qualify_conflicts_field),
            replaces: self.replaces.map(qualify_conflicts_field),
            // Provides are arch-specific: a `foo:amd64` package provides
            // `virt:amd64` (never the `:all` flavour).
            provides: self.provides.map(|f| qualify_provides_field(f, arch)),
            essential: Some(self.essential),
            ..PackageEntry::default()
        }
    }
}

/// Serialize one parsed dependency alternative back to its deb822 text.
fn serialize_dep(dep: &debian_control::lossy::Relation) -> String {
    let mut s = String::new();
    s.push_str(&dep.name);
    if let Some(arch) = &dep.archqual {
        s.push(':');
        s.push_str(arch);
    }
    if let Some((rel, ver)) = &dep.version {
        use std::fmt::Write as _;
        let _ = write!(s, " ({rel} {ver})");
    }
    s
}

fn serialize_dep_groups(groups: &[Vec<debian_control::lossy::Relation>]) -> String {
    groups
        .iter()
        .map(|g| g.iter().map(serialize_dep).collect::<Vec<_>>().join(" | "))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Qualify a dependency field's bare names for architecture `arch`: a bare
/// `libbar` in a package for `arch` means `libbar:arch`, and an
/// `Architecture: all` package (`libbar:all`) also satisfies it — so each
/// bare alternative expands into `libbar:arch | libbar:all`. A bare
/// dependency on a `Multi-Arch: foreign` package (`foreign` names) expands
/// to `libbar:any` instead — any architecture satisfies it. Already
/// qualified names (`:any`, `:all`, `:amd64`, …) are left alone.
fn qualify_dep_field(field: &str, arch: &str, foreign: &HashSet<&str>) -> String {
    let mut groups = crate::parse_dep_groups(field);
    for group in &mut groups {
        let mut expanded = Vec::with_capacity(group.len());
        for dep in group.drain(..) {
            match dep.archqual {
                Some(_) => expanded.push(dep),
                None if foreign.contains(dep.name.as_str()) => {
                    // Multi-Arch: foreign — any architecture satisfies it.
                    expanded.push(debian_control::lossy::Relation {
                        archqual: Some("any".to_string()),
                        ..dep
                    });
                }
                None => {
                    expanded.push(debian_control::lossy::Relation {
                        archqual: Some(arch.to_string()),
                        ..dep.clone()
                    });
                    expanded.push(debian_control::lossy::Relation {
                        archqual: Some("all".to_string()),
                        ..dep
                    });
                }
            }
        }
        *group = expanded;
    }
    serialize_dep_groups(&groups)
}

/// Qualify a Breaks/Conflicts/Replaces field's bare names: an unqualified
/// `Conflicts: dirmngr` conflicts with *every* architecture of dirmngr, so a
/// bare name maps to `dirmngr:any`. Already qualified names are left alone.
fn qualify_conflicts_field(field: &str) -> String {
    let mut groups = crate::parse_dep_groups(field);
    for group in &mut groups {
        for dep in group.iter_mut() {
            if dep.archqual.is_none() {
                dep.archqual = Some("any".to_string());
            }
        }
    }
    serialize_dep_groups(&groups)
}

/// Qualify a Provides field's bare names for `arch`: a `foo:arch` package
/// provides `virt:arch` (never the `:all` flavour). Already qualified names
/// are left alone.
fn qualify_provides_field(field: &str, arch: &str) -> String {
    let mut groups = crate::parse_dep_groups(field);
    for group in &mut groups {
        for dep in group.iter_mut() {
            if dep.archqual.is_none() {
                dep.archqual = Some(arch.to_string());
            }
        }
    }
    serialize_dep_groups(&groups)
}

/// The parsed EDSP input: the request plus the whole package universe.
#[derive(Debug, Clone)]
pub struct EdspInput<'a> {
    pub request: EdspRequest<'a>,
    /// One entry per package version in the universe.
    pub versions: Vec<EdspVersion<'a>>,
}

/// One `Install:` / `Remove:` / `Autoremove:` stanza of the solution.
#[derive(Debug, Clone)]
pub struct SolutionStanza {
    /// The `APT-ID` referencing a version in the universe — the field apt
    /// acts on.
    pub apt_id: u64,
    /// Package name (verbose field, recommended for readability).
    pub package: String,
    /// Version (verbose field).
    pub version: String,
    /// Architecture (verbose field).
    pub architecture: Option<String>,
}

/// The solution apt should apply.
#[derive(Debug, Clone, Default)]
pub struct EdspSolution {
    /// `Install:` stanzas — new packages / upgrades / downgrades.
    pub installs: Vec<SolutionStanza>,
    /// `Remove:` stanzas.
    pub removes: Vec<SolutionStanza>,
    /// `Autoremove:` stanzas — packages apt should mark as auto-removable.
    pub autoremove: Vec<SolutionStanza>,
}

/// Errors while speaking EDSP.
#[derive(Debug, thiserror::Error)]
pub enum EdspError {
    #[error("malformed EDSP request: {0}")]
    Request(String),
    #[error("malformed package stanza: {0}")]
    Package(String),
    #[error(transparent)]
    Resolve(#[from] crate::apt_provider::ResolveError),
}

/// Parse the EDSP request + universe from `input` (apt's stdin payload).
///
/// The request and every version borrow from `input` — nothing is copied.
pub fn parse_input(input: &str) -> Result<EdspInput<'_>, EdspError> {
    let mut paragraphs = input.split("\n\n");
    let request_para = paragraphs
        .next()
        .ok_or_else(|| EdspError::Request("empty input".to_string()))?;
    let request = parse_request(request_para)?;

    let mut versions = Vec::new();
    for para in paragraphs {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        versions.push(parse_version(para)?);
    }

    Ok(EdspInput { request, versions })
}

fn parse_request(para: &str) -> Result<EdspRequest<'_>, EdspError> {
    let mut request = EdspRequest {
        architecture: "",
        install: Vec::new(),
        remove: Vec::new(),
        upgrade_all: false,
        forbid_new_install: false,
        forbid_remove: false,
        autoremove: false,
    };
    let mut saw_request = false;
    for line in para.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "Request" => saw_request = true,
            "Architecture" => request.architecture = value,
            "Install" => request.install.extend(value.split_whitespace()),
            "Remove" => request.remove.extend(value.split_whitespace()),
            "Upgrade" | "Upgrade-All" | "Dist-Upgrade" => {
                if value.eq_ignore_ascii_case("yes") {
                    request.upgrade_all = true;
                }
            }
            "Forbid-New-Install" => request.forbid_new_install = value.eq_ignore_ascii_case("yes"),
            "Forbid-Remove" => request.forbid_remove = value.eq_ignore_ascii_case("yes"),
            "Autoremove" => request.autoremove = value.eq_ignore_ascii_case("yes"),
            // "Architectures" / "Machine-ID" / "Solver" are informational.
            _ => {}
        }
    }
    if !saw_request {
        return Err(EdspError::Request("missing Request: field".to_string()));
    }
    Ok(request)
}

/// Parse one package-version paragraph of the universe. EDSP paragraphs are
/// plain single-line deb822 fields, so this scans the lines directly (like
/// [`parse_request`]) and borrows every value from `para` — no intermediate
/// owned [`Deb822`](deb822_fast::Deb822) to copy out of.
fn parse_version(para: &str) -> Result<EdspVersion<'_>, EdspError> {
    let mut package = None;
    let mut version = None;
    let mut architecture = None;
    let mut apt_id = None;
    let mut installed = false;
    let mut hold = false;
    let mut essential = false;
    let mut apt_candidate = false;
    let mut apt_automatic = false;
    let mut depends = None;
    let mut pre_depends = None;
    let mut recommends = None;
    let mut suggests = None;
    let mut breaks = None;
    let mut conflicts = None;
    let mut replaces = None;
    let mut provides = None;
    let mut multi_arch = None;
    for line in para.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "Package" => package = Some(value),
            "Version" => version = Some(value),
            "Architecture" => architecture = Some(value),
            "APT-ID" => apt_id = value.parse().ok(),
            "Installed" => installed = value.eq_ignore_ascii_case("yes"),
            "Hold" => hold = value.eq_ignore_ascii_case("yes"),
            "Essential" => essential = value.eq_ignore_ascii_case("yes"),
            "APT-Candidate" => apt_candidate = value.eq_ignore_ascii_case("yes"),
            "APT-Automatic" => apt_automatic = value.eq_ignore_ascii_case("yes"),
            "Depends" => depends = Some(value),
            "Pre-Depends" => pre_depends = Some(value),
            "Recommends" => recommends = Some(value),
            "Suggests" => suggests = Some(value),
            "Breaks" => breaks = Some(value),
            "Conflicts" => conflicts = Some(value),
            "Replaces" => replaces = Some(value),
            "Provides" => provides = Some(value),
            "Multi-Arch" => multi_arch = Some(value),
            _ => {}
        }
    }
    Ok(EdspVersion {
        package: package.ok_or_else(|| EdspError::Package("missing Package: field".to_string()))?,
        version: version.ok_or_else(|| EdspError::Package("missing Version: field".to_string()))?,
        architecture: architecture
            .ok_or_else(|| EdspError::Package("missing Architecture: field".to_string()))?,
        apt_id: apt_id
            .ok_or_else(|| EdspError::Package("missing or invalid APT-ID field".to_string()))?,
        installed,
        hold,
        essential,
        apt_candidate,
        apt_automatic,
        depends,
        pre_depends,
        recommends,
        suggests,
        breaks,
        conflicts,
        replaces,
        provides,
        multi_arch,
    })
}

/// The EDSP universe as an index.
///
/// Wraps an [`AptDb`] built from the universe (which merges versions and
/// provides the usual lookups) and overrides candidate selection to return
/// the version apt marked `APT-Candidate: yes` — apt's pins are already
/// baked into that marker, so a pinned package resolves like apt would,
/// instead of always taking the plain highest version.
///
/// The solver and planner consume an [`AptDb`] directly; this wrapper only
/// carries the candidate override, which the mark phase ([`solve_with`])
/// uses to pin each install root to apt's candidate.
struct EdspIndex<'a> {
    db: AptDb,
    /// base name → the (arch, version) apt marked `APT-Candidate: yes`,
    /// borrowing from the parsed universe.
    candidates: HashMap<&'a str, Vec<(&'a str, &'a str)>>,
}

impl<'a> EdspIndex<'a> {
    fn new(db: AptDb, candidates: HashMap<&'a str, Vec<(&'a str, &'a str)>>) -> Self {
        Self { db, candidates }
    }

    /// The candidate version of `name`: the version apt marked
    /// `APT-Candidate: yes` when there is one, otherwise the plain highest
    /// version. The mark phase pins each install root to this version.
    fn candidate_version(&self, name: &str) -> Option<Cow<'_, PackageVersion>> {
        // A request for `foo:amd64` may be served by the `Architecture: all`
        // package `foo:all` — apt installs all-arch packages for the
        // requested arch. Try the exact name first, then the `:all` flavour.
        let all_name = name.rsplit_once(':').map(|(base, _)| format!("{base}:all"));
        for cand_name in std::iter::once(name).chain(all_name.as_deref()) {
            // The candidate index is keyed by base name; split the fullname
            // to find the (arch, version) apt marked as candidate.
            let version = cand_name.rsplit_once(':').and_then(|(base, arch)| {
                self.candidates
                    .get(base)
                    .and_then(|vs| vs.iter().find(|&&(a, _)| a == arch))
                    .map(|&(_, v)| v)
            });
            if let Some(version) = version
                && let Some(entry) = self.db.get_version(cand_name, version)
            {
                return Some(entry);
            }
        }
        self.db.candidate_version(name).or_else(|| {
            all_name
                .as_deref()
                .and_then(|n| self.db.candidate_version(n))
        })
    }
}

/// Solve an [`EdspInput`] with apt's default resolution options (install
/// Recommends, no Suggests, prefer installed versions) and no
/// `APT::NeverAutoRemove` patterns.
pub fn solve(input: &EdspInput) -> Result<EdspSolution, EdspError> {
    solve_with(input, ResolveOptions::default(), &[])
}

/// Like [`solve`], with explicit resolution options and `APT::NeverAutoRemove`
/// patterns. The caller resolves those from its configuration (the
/// `oma-edsp` binary loads the system apt config); the resolver itself takes
/// plain values and never touches apt configuration.
pub fn solve_with(
    input: &EdspInput,
    options: ResolveOptions,
    never_auto_remove: &[String],
) -> Result<EdspSolution, EdspError> {
    // Optional phase timing (diagnostic): set `OMA_EDSP_TIMING=1` to print
    // per-phase elapsed time to stderr, like apt's `Debug::APT::Solver`.
    let profile = std::env::var_os("OMA_EDSP_TIMING").is_some();
    let mut t = std::time::Instant::now();
    let mut phase = |name: &str| {
        if profile {
            eprintln!(
                "oma-edsp[{name}] {:.1}ms",
                t.elapsed().as_secs_f64() * 1000.0
            );
        }
        t = std::time::Instant::now();
    };

    // Repository index: the universe, with APT-Candidate honoured. Package
    // identity is the arch-qualified name, so `foo:amd64` and `foo:i386` are
    // distinct packages and never merge.
    //
    // `Multi-Arch: foreign` names: a bare dependency on one of these may be
    // satisfied by any architecture, so `to_entry` needs to know them while
    // qualifying dependencies.
    let foreign: HashSet<&str> = input
        .versions
        .iter()
        .filter(|v| v.multi_arch == Some("foreign"))
        .map(|v| v.package)
        .collect();
    let entries: Vec<PackageEntry> = input
        .versions
        .iter()
        .map(|v| v.to_entry(input.request.architecture, &foreign))
        .collect();
    let mut candidates: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
    for v in input.versions.iter().filter(|v| v.apt_candidate) {
        candidates
            .entry(v.package)
            .or_default()
            .push((v.architecture, v.version));
    }
    let db = AptDb::from_entries(input.request.architecture, entries);
    let index = EdspIndex::new(db, candidates);
    phase("index");

    // Installed state comes from the universe, not /var/lib/dpkg/status.
    let dpkg = build_dpkg_state(&input.versions);

    // Auto-installed set (installed packages marked APT-Automatic: yes).
    let auto_installed: HashSet<String> = input
        .versions
        .iter()
        .filter(|v| v.installed && v.apt_automatic)
        .map(|v| v.fullname())
        .collect();

    // Version metadata index: base name → the versions of that package
    // across all archs/versions, borrowing from the universe — built once
    // so the solution loop never rescans it per stanza. Fullnames are
    // split into (base, arch) at lookup time.
    let mut apt_id_of: HashMap<&str, Vec<VersionMeta<'_>>> = HashMap::new();
    for v in &input.versions {
        apt_id_of.entry(v.package).or_default().push(VersionMeta {
            apt_id: v.apt_id,
            architecture: v.architecture,
            version: v.version,
        });
    }
    phase("state");

    let mut planner = TransactionPlanner::new(&index.db, &dpkg, options);

    if input.request.upgrade_all {
        let mode = if input.request.forbid_new_install && input.request.forbid_remove {
            UpgradeMode::MinimalUpgrade
        } else if input.request.forbid_remove {
            UpgradeMode::SafeUpgrade
        } else {
            UpgradeMode::FullUpgrade
        };
        planner.upgrade(mode);
    }

    for name in &input.request.install {
        // Request names are arch-qualified and match the index directly.
        // Candidate selection honours apt's `APT-Candidate` marker, so a
        // pinned package is marked at the version apt's policy chose.
        let candidate = index.candidate_version(name).ok_or_else(|| {
            EdspError::Package(format!("requested package {name} is not in the universe"))
        })?;
        planner.mark_install(candidate.into_owned().entry, false);
    }
    for name in &input.request.remove {
        planner.mark_remove(PackageEntry {
            package: (*name).to_string(),
            ..PackageEntry::default()
        });
    }
    phase("mark");

    let changeset = planner.resolve()?;
    phase("resolve");

    let mut installs = Vec::new();
    let mut removes = Vec::new();
    let mut autoremove = Vec::new();

    for change in changeset.get_changes() {
        match change.kind {
            ChangeKind::Install
            | ChangeKind::Upgrade
            | ChangeKind::Downgrade
            | ChangeKind::Reinstall => {
                if let Some(version) = &change.to_version
                    && let Some(stanza) = stanza_for(&change.package, version, &apt_id_of)
                {
                    installs.push(stanza);
                }
            }
            ChangeKind::Remove => {
                if let Some(version) = &change.from_version
                    && let Some(stanza) = stanza_for(&change.package, version, &apt_id_of)
                {
                    removes.push(stanza);
                }
            }
        }
    }

    // Autoremove stanzas: newly auto-installed packages (so apt tracks them
    // for a future autoremove), and — when apt asked — the installed
    // auto-installed packages that are no longer needed.
    for change in changeset.get_changes() {
        if change.auto_installed
            && let Some(version) = &change.to_version
            && let Some(stanza) = stanza_for(&change.package, version, &apt_id_of)
        {
            autoremove.push(stanza);
        }
    }
    if input.request.autoremove {
        for name in planner.autoremove(&auto_installed, never_auto_remove) {
            // `name` is an arch-qualified fullname; split it to find the
            // installed version's metadata in the index.
            let (base, arch) = name.rsplit_once(':').unwrap_or((&name, ""));
            let Some(version) = dpkg.installed_version(&name) else {
                continue;
            };
            if let Some(meta) = apt_id_of.get(base).and_then(|vs| {
                vs.iter()
                    .find(|m| m.architecture == arch && m.version == version)
            }) {
                autoremove.push(SolutionStanza {
                    apt_id: meta.apt_id,
                    package: name,
                    version: version.to_string(),
                    architecture: Some(meta.architecture.to_string()),
                });
            }
        }
    }

    phase("stanzas");
    Ok(EdspSolution {
        installs,
        removes,
        autoremove,
    })
}

/// The universe's metadata for one package version: the `APT-ID` the
/// solution references, plus the version/architecture needed to build the
/// verbose solution fields. Borrows from the parsed universe.
struct VersionMeta<'a> {
    apt_id: u64,
    architecture: &'a str,
    version: &'a str,
}

/// Build a [`SolutionStanza`] for `(package, version)` by looking up the
/// version's metadata in the index; `None` when the universe has no such
/// version (should not happen for a solution the resolver produced from
/// that universe).
fn stanza_for<'a>(
    package: &str,
    version: &str,
    apt_id_of: &HashMap<&'a str, Vec<VersionMeta<'a>>>,
) -> Option<SolutionStanza> {
    let (base, arch) = package.rsplit_once(':')?;
    let meta = apt_id_of
        .get(base)?
        .iter()
        .find(|m| m.architecture == arch && m.version == version)?;
    Some(SolutionStanza {
        apt_id: meta.apt_id,
        package: package.to_string(),
        version: version.to_string(),
        architecture: Some(meta.architecture.to_string()),
    })
}

/// Build the installed state from the EDSP universe: every version marked
/// `Installed: yes` is installed at that version, with `Hold` / `Essential`
/// carried over (held packages keep their `Status: hold ok installed`, which
/// the planner's essential/protected/held protection reads). Built directly
/// from the universe — no `/var/lib/dpkg/status` file and no synthesized
/// deb822 text to re-parse.
fn build_dpkg_state(versions: &[EdspVersion<'_>]) -> DpkgState {
    let mut index = DpkgIndex::default();
    for v in versions {
        if v.installed {
            index.add_installed(v.fullname(), v.version.to_string(), v.essential, v.hold);
        }
    }
    DpkgState::from_index(index)
}

/// Write the solution stanzas apt expects on stdout.
pub fn write_solution<W: Write>(out: &mut W, solution: &EdspSolution) -> std::io::Result<()> {
    for s in &solution.installs {
        write_stanza(out, "Install", s)?;
    }
    for s in &solution.removes {
        write_stanza(out, "Remove", s)?;
    }
    for s in &solution.autoremove {
        write_stanza(out, "Autoremove", s)?;
    }
    Ok(())
}

fn write_stanza<W: Write>(out: &mut W, field: &str, s: &SolutionStanza) -> std::io::Result<()> {
    // The APT-ID field is what apt acts on; Package/Version/Architecture are
    // verbose fields the protocol recommends for readability.
    writeln!(out, "{field}: {}", s.apt_id)?;
    writeln!(out, "Package: {}", s.package)?;
    writeln!(out, "Version: {}", s.version)?;
    if let Some(arch) = &s.architecture {
        writeln!(out, "Architecture: {arch}")?;
    }
    writeln!(out)?;
    Ok(())
}

/// Write an `Error:` stanza — apt displays `Message:` to the user and treats
/// the solve as failed.
pub fn write_error<W: Write>(out: &mut W, code: &str, message: &str) -> std::io::Result<()> {
    writeln!(out, "Error: {code}")?;
    writeln!(out, "Message: {message}")?;
    writeln!(out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small universe: foo 1.0 (newest, depends on libbar >= 2.0) and
    /// foo 0.9 (installed); libbar 2.5 (candidate), 2.0 and 1.0 (installed).
    fn universe() -> String {
        "\
Package: foo
Version: 1.0
Architecture: amd64
APT-ID: 100
Depends: libbar (>= 2.0)

Package: foo
Version: 0.9
Architecture: amd64
APT-ID: 101
Installed: yes

Package: libbar
Version: 2.5
Architecture: amd64
APT-ID: 200
APT-Candidate: yes

Package: libbar
Version: 2.0
Architecture: amd64
APT-ID: 201

Package: libbar
Version: 1.0
Architecture: amd64
APT-ID: 202
Installed: yes
"
        .to_string()
    }

    /// Build a test universe. The text is leaked so the returned input can
    /// borrow it with a static lifetime — fine for tests.
    fn input(request: &str) -> EdspInput<'static> {
        let text = format!(
            "Request: EDSP 0.5\nArchitecture: amd64\n{request}\n\n{}",
            universe()
        );
        parse_input(Box::leak(text.into_boxed_str())).unwrap()
    }

    #[test]
    fn test_parse_input() {
        let input = input("Install: foo:amd64");
        assert_eq!(input.request.architecture, "amd64");
        assert_eq!(input.request.install, vec!["foo:amd64"]);
        assert_eq!(input.versions.len(), 5);
        assert_eq!(input.versions[0].apt_id, 100);
        assert!(input.versions[1].installed);
    }

    #[test]
    fn test_solve_install() {
        let input = input("Install: foo:amd64");
        let solution = solve(&input).unwrap();
        // foo must go to 1.0 and pull libbar 2.5 (the candidate).
        let foo = solution
            .installs
            .iter()
            .find(|s| s.package == "foo:amd64")
            .expect("foo in solution");
        assert_eq!(foo.version, "1.0");
        assert_eq!(foo.apt_id, 100);
        let bar = solution
            .installs
            .iter()
            .find(|s| s.package == "libbar:amd64")
            .expect("libbar in solution");
        assert_eq!(bar.version, "2.5");
        assert_eq!(bar.apt_id, 200);
    }

    #[test]
    fn test_solve_remove() {
        let input = input("Remove: foo:amd64");
        let solution = solve(&input).unwrap();
        assert_eq!(solution.removes.len(), 1);
        assert_eq!(solution.removes[0].package, "foo:amd64");
        assert_eq!(solution.removes[0].apt_id, 101);
    }

    #[test]
    fn test_solve_upgrade_all() {
        let input = input("Upgrade-All: yes");
        let solution = solve(&input).unwrap();
        // libbar is installed at 1.0 and the candidate is 2.5.
        assert!(
            solution
                .installs
                .iter()
                .any(|s| s.package == "libbar:amd64" && s.version == "2.5"),
            "libbar upgraded to the candidate: {:?}",
            solution.installs
        );
    }

    #[test]
    fn test_solve_unknown_package() {
        assert!(solve(&input("Install: nonexistent:amd64")).is_err());
    }

    #[test]
    fn test_write_solution() {
        let solution = EdspSolution {
            installs: vec![SolutionStanza {
                apt_id: 100,
                package: "foo".to_string(),
                version: "1.0".to_string(),
                architecture: Some("amd64".to_string()),
            }],
            ..EdspSolution::default()
        };
        let mut out = Vec::new();
        write_solution(&mut out, &solution).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with("Install: 100\nPackage: foo\nVersion: 1.0\n"));
    }

    /// Multi-arch: `foo:amd64` and `foo:i386` are distinct packages even at
    /// the same version — installing `foo:amd64` must not pull the i386
    /// flavour.
    #[test]
    fn test_multiarch_not_merged() {
        let input = parse_input(
            "Request: EDSP 0.5\nArchitecture: amd64\nInstall: foo:amd64\n\n\
             Package: foo\nVersion: 1.0\nArchitecture: amd64\nAPT-ID: 10\n\
             APT-Candidate: yes\nDepends: libbar (>= 2.0)\n\n\
             Package: foo\nVersion: 1.0\nArchitecture: i386\nAPT-ID: 20\n\
             APT-Candidate: yes\n\n\
             Package: libbar\nVersion: 2.0\nArchitecture: amd64\nAPT-ID: 30\n\
             APT-Candidate: yes\n\n",
        )
        .unwrap();
        let solution = solve(&input).unwrap();
        let installed: Vec<_> = solution.installs.iter().map(|s| s.apt_id).collect();
        assert!(
            installed.contains(&10),
            "foo:amd64 installed: {installed:?}"
        );
        assert!(
            installed.contains(&30),
            "libbar:amd64 installed: {installed:?}"
        );
        assert!(
            !installed.contains(&20),
            "foo:i386 must not be installed: {installed:?}"
        );
    }

    /// An `Architecture: all` package satisfies a dependency from any
    /// arch-qualified flavour: `foo:amd64`'s dependency on `libbaz` resolves
    /// to the `libbaz:all` package.
    #[test]
    fn test_all_arch_satisfies_dep() {
        let input = parse_input(
            "Request: EDSP 0.5\nArchitecture: amd64\nInstall: foo:amd64\n\n\
             Package: foo\nVersion: 1.0\nArchitecture: amd64\nAPT-ID: 10\n\
             APT-Candidate: yes\nDepends: libbaz\n\n\
             Package: libbaz\nVersion: 1.0\nArchitecture: all\nAPT-ID: 20\n\
             APT-Candidate: yes\n\n",
        )
        .unwrap();
        let solution = solve(&input).unwrap();
        assert!(
            solution
                .installs
                .iter()
                .any(|s| s.package == "libbaz:all" && s.apt_id == 20),
            "libbaz:all pulled in: {:?}",
            solution.installs
        );
    }

    /// A transitional package that `Provides` the very name it also
    /// `Conflicts` with (like `gnupg` providing + conflicting `dirmngr`)
    /// must not constrain itself away — apt treats that as a self-conflict
    /// and drops it. The bare `Conflicts: dirmngr` is arch-unconstrained
    /// (`dirmngr:any`), and the holder's own `dirmngr:amd64` Provides must
    /// be recognised through the `:any` alias.
    #[test]
    fn test_self_conflict_on_provided_virtual() {
        let input = parse_input(
            "Request: EDSP 0.5\nArchitecture: amd64\nInstall: app:amd64\n\n\
             Package: app\nVersion: 1.0\nArchitecture: amd64\nAPT-ID: 10\n\
             APT-Candidate: yes\nDepends: apt\n\n\
             Package: apt\nVersion: 2.0\nArchitecture: amd64\nAPT-ID: 20\n\
             APT-Candidate: yes\nDepends: gnupg (>= 2:2.5.18)\n\n\
             Package: gnupg\nVersion: 2:2.5.21\nArchitecture: amd64\nAPT-ID: 30\n\
             APT-Candidate: yes\nConflicts: dirmngr\nProvides: dirmngr\n\n",
        )
        .unwrap();
        let solution = solve(&input).unwrap();
        let installed: Vec<_> = solution
            .installs
            .iter()
            .map(|s| s.package.clone())
            .collect();
        assert!(
            installed.contains(&"gnupg:amd64".to_string()),
            "gnupg installed despite self-conflict: {installed:?}"
        );
        assert!(
            installed.contains(&"apt:amd64".to_string()),
            "apt installed: {installed:?}"
        );
        assert!(
            installed.contains(&"app:amd64".to_string()),
            "app installed: {installed:?}"
        );
    }

    /// An explicit `:any` dependency is satisfied by the `Architecture: all`
    /// package too: `libbaz:any` resolves to `libbaz:all` (the `:any` alias
    /// covers every architecture *and* the all-arch flavour).
    #[test]
    fn test_any_dep_matches_all_package() {
        let input = parse_input(
            "Request: EDSP 0.5\nArchitecture: amd64\nInstall: app:amd64\n\n\
             Package: app\nVersion: 1.0\nArchitecture: amd64\nAPT-ID: 10\n\
             APT-Candidate: yes\nDepends: libbaz:any\n\n\
             Package: libbaz\nVersion: 1.0\nArchitecture: all\nAPT-ID: 20\n\
             APT-Candidate: yes\n\n",
        )
        .unwrap();
        let solution = solve(&input).unwrap();
        assert!(
            solution
                .installs
                .iter()
                .any(|s| s.package == "libbaz:all" && s.apt_id == 20),
            "libbaz:all satisfies libbaz:any: {:?}",
            solution.installs
        );
    }

    /// `Multi-Arch: foreign`: an amd64 package's bare dependency on a
    /// foreign package is satisfied by the i386 flavour of it — the bare
    /// name qualifies to `bar:any`, not `bar:amd64 | bar:all`.
    #[test]
    fn test_foreign_bare_dep_crosses_arch() {
        let input = parse_input(
            "Request: EDSP 0.5\nArchitecture: amd64\nInstall: app:amd64\n\n\
             Package: app\nVersion: 1.0\nArchitecture: amd64\nAPT-ID: 10\n\
             APT-Candidate: yes\nDepends: bar\n\n\
             Package: bar\nVersion: 1.0\nArchitecture: i386\n\
             Multi-Arch: foreign\nAPT-ID: 20\nAPT-Candidate: yes\n\n",
        )
        .unwrap();
        let solution = solve(&input).unwrap();
        assert!(
            solution
                .installs
                .iter()
                .any(|s| s.package == "bar:i386" && s.apt_id == 20),
            "foreign bar:i386 satisfies app's bare dependency: {:?}",
            solution.installs
        );
    }

    /// A non-foreign package does *not* satisfy a bare dependency across
    /// architectures: `bar:i386` alone cannot serve `app:amd64`'s `Depends:
    /// bar` (no `Multi-Arch: foreign`).
    #[test]
    fn test_non_foreign_bare_dep_stays_same_arch() {
        let input = parse_input(
            "Request: EDSP 0.5\nArchitecture: amd64\nInstall: app:amd64\n\n\
             Package: app\nVersion: 1.0\nArchitecture: amd64\nAPT-ID: 10\n\
             APT-Candidate: yes\nDepends: bar\n\n\
             Package: bar\nVersion: 1.0\nArchitecture: i386\nAPT-ID: 20\n\
             APT-Candidate: yes\n\n",
        )
        .unwrap();
        assert!(
            solve(&input).is_err(),
            "non-foreign bar:i386 must not satisfy app:amd64's bare dependency"
        );
    }

    /// apt's `AddImplicitDepends`: two group members of different
    /// architectures cannot both be installed (no `Multi-Arch: same`).
    #[test]
    fn test_group_members_exclude_each_other() {
        let input = parse_input(
            "Request: EDSP 0.5\nArchitecture: amd64\nInstall: app1:amd64 app2:amd64\n\n\
             Package: app1\nVersion: 1.0\nArchitecture: amd64\nAPT-ID: 10\n\
             APT-Candidate: yes\nDepends: foo:amd64\n\n\
             Package: app2\nVersion: 1.0\nArchitecture: amd64\nAPT-ID: 11\n\
             APT-Candidate: yes\nDepends: foo:i386\n\n\
             Package: foo\nVersion: 1.0\nArchitecture: amd64\nAPT-ID: 20\n\
             APT-Candidate: yes\n\n\
             Package: foo\nVersion: 1.0\nArchitecture: i386\nAPT-ID: 21\n\
             APT-Candidate: yes\n\n",
        )
        .unwrap();
        assert!(
            solve(&input).is_err(),
            "foo:amd64 and foo:i386 (non-same) must be mutually exclusive"
        );
    }

    /// `Multi-Arch: same` members co-install at the same version.
    #[test]
    fn test_same_members_coinstall_same_version() {
        let input = parse_input(
            "Request: EDSP 0.5\nArchitecture: amd64\nInstall: app1:amd64 app2:amd64\n\n\
             Package: app1\nVersion: 1.0\nArchitecture: amd64\nAPT-ID: 10\n\
             APT-Candidate: yes\nDepends: foo:amd64\n\n\
             Package: app2\nVersion: 1.0\nArchitecture: amd64\nAPT-ID: 11\n\
             APT-Candidate: yes\nDepends: foo:i386\n\n\
             Package: foo\nVersion: 1.0\nArchitecture: amd64\n\
             Multi-Arch: same\nAPT-ID: 20\nAPT-Candidate: yes\n\n\
             Package: foo\nVersion: 1.0\nArchitecture: i386\n\
             Multi-Arch: same\nAPT-ID: 21\nAPT-Candidate: yes\n\n",
        )
        .unwrap();
        let solution = solve(&input).unwrap();
        let installed: Vec<_> = solution.installs.iter().map(|s| s.apt_id).collect();
        assert!(
            installed.contains(&20) && installed.contains(&21),
            "same foo:amd64 and foo:i386 coexist at version 1.0: {installed:?}"
        );
    }

    /// `Multi-Arch: same` members are mutually exclusive at different
    /// versions.
    #[test]
    fn test_same_members_exclude_different_versions() {
        let input = parse_input(
            "Request: EDSP 0.5\nArchitecture: amd64\nInstall: app1:amd64 app2:amd64\n\n\
             Package: app1\nVersion: 1.0\nArchitecture: amd64\nAPT-ID: 10\n\
             APT-Candidate: yes\nDepends: foo:amd64\n\n\
             Package: app2\nVersion: 1.0\nArchitecture: amd64\nAPT-ID: 11\n\
             APT-Candidate: yes\nDepends: foo:i386\n\n\
             Package: foo\nVersion: 1.0\nArchitecture: amd64\n\
             Multi-Arch: same\nAPT-ID: 20\nAPT-Candidate: yes\n\n\
             Package: foo\nVersion: 2.0\nArchitecture: i386\n\
             Multi-Arch: same\nAPT-ID: 21\nAPT-Candidate: yes\n\n",
        )
        .unwrap();
        assert!(
            solve(&input).is_err(),
            "same foo:amd64-1.0 and foo:i386-2.0 must be mutually exclusive"
        );
    }
}
