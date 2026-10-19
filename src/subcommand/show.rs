use std::io::Write;
use std::{borrow::Cow, io::stdout};

use anyhow::Context;
use clap::Args;
use clap_complete::ArgValueCompleter;
use dialoguer::console::{StyledObject, style};
use oma_apt_pkg::apt_sources::{IndexTargetTemplates, substitute};
use oma_apt_pkg::{
    AptConfig, AptExtendedStates, AptState, Control, Package, ResolvedPackage, Source, Version,
};
use oma_logger::info;
use oma_utils::human_bytes::HumanBytes;
use serde::Serialize;

use crate::{
    args::CliExecuter, completions::pkgnames_and_path_completions, config::OmaConfig,
    error::OutputError, exit_handle::ExitHandle, fl,
};

use super::utils::handle_no_result;

#[derive(Debug, Args)]
pub struct Show {
    /// Package(s) to show
    #[arg(required = true, add = ArgValueCompleter::new(pkgnames_and_path_completions), help = fl!("clap-show-packages-help"))]
    #[arg(help_heading = &**crate::args::ARG_HELP_HEADING_MUST)]
    packages: Vec<String>,
    /// Show information on all available version(s) of (a) package(s) from all repository(ies)
    #[arg(short, long, help = fl!("clap-show-all-help"))]
    all: bool,
    /// Set output format as JSON
    #[arg(long, help = fl!("clap-json-help"))]
    json: bool,
}

/// Ordered list of (label, field accessor) for display.
const DISPLAY_FIELDS: &[(&str, &str)] = &[
    ("Package:", "Package"),
    ("Version:", "Version"),
    ("Section:", "Section"),
    ("Maintainer:", "Maintainer"),
    ("Installed-Size:", "Installed-Size"),
    ("Pre-Depends:", "Pre-Depends"),
    ("Depends:", "Depends"),
    ("Breaks:", "Breaks"),
    ("Conflicts:", "Conflicts"),
    ("Replaces:", "Replaces"),
    ("Recommends:", "Recommends"),
    ("Suggests:", "Suggests"),
    ("Provides:", "Provides"),
    ("Download-Size:", "Size"),
    ("Description:", "Description"),
];

impl CliExecuter for Show {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let Show {
            all,
            json,
            packages,
        } = self;

        let apt_cfg = config.apt_config();
        let (mut state, ext_states) = load_state(apt_cfg)?;

        // Local `.deb` files insert into the state; `resolve` then finds
        // them without needing a mutable borrow.
        for path in packages
            .iter()
            .filter(|query| query.ends_with(".deb") && std::path::Path::new(query).is_file())
        {
            state
                .insert_deb(path)
                .map_err(|e| OutputError::with_source("Failed to read local package", e))?;
        }

        // Resolve each query against the unified package state — apt lists
        // plus the live dpkg status, selected by architecture the way apt
        // selects a package.
        let resolution = state.resolve(packages);

        handle_no_result(
            resolution.no_match.iter().map(String::as_str).collect(),
            config.no_progress(),
        )?;

        let mut stdout = stdout();

        for (i, resolved) in resolution.resolved.iter().enumerate() {
            display_group(
                &mut stdout,
                resolved,
                &state,
                &ext_states,
                all,
                json,
                apt_cfg,
            )?;

            if i != resolution.resolved.len() - 1 {
                writeln!(stdout).ok();
            }

            // Show "N additional versions" hint for a single package without
            // --all. The count comes from the package the query selected, so
            // `foo:amd64` only advertises versions of that architecture.
            let additional = if !all && !json && resolution.resolved.len() == 1 {
                state
                    .package(resolved.key.name(), resolved.key.arch())
                    .map_or(0, |pkg| pkg.version_count().saturating_sub(1))
            } else {
                0
            };

            if additional > 0 {
                info!("{}", fl!("additional-version", len = additional));
            }
        }

        Ok(ExitHandle::default())
    }
}

/// Display one resolved query, honoring the JSON flag.
fn display_group(
    stdout: &mut impl Write,
    resolved: &ResolvedPackage,
    state: &AptState,
    ext_states: &AptExtendedStates,
    all: bool,
    json: bool,
    apt_cfg: &AptConfig,
) -> Result<(), OutputError> {
    let Some(pkg) = state.package(resolved.key.name(), resolved.key.arch()) else {
        return Ok(());
    };

    // `--all` on a local `.deb` shows every version the package has in the
    // database, not just the version read from the file. Without `--all`
    // the default stays on the file's version.
    let is_local = resolved.versions.iter().all(|version| {
        version
            .sources
            .iter()
            .any(|source| source.suite == "local-deb")
    });
    let versions: &[Version] = if all && is_local {
        pkg.versions()
    } else {
        &resolved.versions
    };

    if json {
        display_versions_to_json(stdout, resolved, versions, state, apt_cfg)?;
    } else {
        display_versions(stdout, &pkg, versions, state, ext_states, all, apt_cfg);
    }
    Ok(())
}

/// The version displayed without `--all`: the highest one of the package the
/// query selected. Architecture was already decided by [`AptState::resolve`].
fn default_version(versions: &[Version]) -> Option<&Version> {
    versions
        .iter()
        .max_by(|a, b| a.parsed_version().cmp(&b.parsed_version()))
}

fn display_versions(
    stdout: &mut impl Write,
    pkg: &Package<'_>,
    versions: &[Version],
    state: &AptState,
    ext_states: &AptExtendedStates,
    show_all: bool,
    apt_cfg: &AptConfig,
) {
    if !show_all && let Some(version) = default_version(versions) {
        display_version(stdout, pkg, version, state, ext_states, apt_cfg);
        return;
    }

    // Show all versions oldest → newest, comparing the cached parsed
    // versions instead of re-parsing each version string. Unstable is fine:
    // ties only occur between versions that compare equal (e.g. unparsable
    // version strings), whose display order is meaningless.
    let mut shown: Vec<&Version> = versions.iter().collect();
    shown.sort_unstable_by_key(|a| a.parsed_version());

    for (idx, version) in shown.iter().enumerate() {
        if idx != 0 {
            writeln!(stdout).ok();
        }

        display_version(stdout, pkg, version, state, ext_states, apt_cfg);
    }
}

/// Resolve one display field from a package entry, in the order of
/// [`DISPLAY_FIELDS`].
fn field_value<'a>(entry: &'a Control, field: &str) -> Option<Cow<'a, str>> {
    match field {
        "Package" => Some(Cow::Borrowed(&entry.package)),
        "Version" => entry.version.as_deref().map(Cow::Borrowed),
        "Section" => entry.section.as_deref().map(Cow::Borrowed),
        "Maintainer" => entry.maintainer.as_deref().map(Cow::Borrowed),
        "Installed-Size" => entry
            .installed_size
            .map(|s| Cow::Owned(HumanBytes(s * 1024).to_string())),
        "Pre-Depends" => entry.pre_depends.as_deref().map(Cow::Borrowed),
        "Depends" => entry.depends.as_deref().map(Cow::Borrowed),
        "Breaks" => entry.breaks.as_deref().map(Cow::Borrowed),
        "Conflicts" => entry.conflicts.as_deref().map(Cow::Borrowed),
        "Replaces" => entry.replaces.as_deref().map(Cow::Borrowed),
        "Recommends" => entry.recommends.as_deref().map(Cow::Borrowed),
        "Suggests" => entry.suggests.as_deref().map(Cow::Borrowed),
        "Provides" => entry.provides.as_deref().map(Cow::Borrowed),
        "Size" => entry.size.map(|s| Cow::Owned(HumanBytes(s).to_string())),
        "Description" => entry.description.as_deref().map(Cow::Borrowed),
        _ => None,
    }
}

/// Display one version as a single block, listing every source it is
/// available from. The package-level fields (name, installed state) come
/// from the [`Package`] view, the rest from this specific version.
fn display_version(
    stdout: &mut impl Write,
    pkg: &Package<'_>,
    version: &Version,
    state: &AptState,
    ext_states: &AptExtendedStates,
    apt_cfg: &AptConfig,
) {
    for (label, field) in DISPLAY_FIELDS {
        // The package name is this version's fullname (`name:arch`,
        // omitting the native arch), like apt's `apt show` — taken from
        // the displayed version so an arch-filtered query (`foo:i386`) or
        // `--all` labels each block with its own architecture.
        let value = if *field == "Package" {
            Some(pkg.fullname(version, true))
        } else {
            field_value(&version.control, field)
        };
        let Some(value) = value else {
            continue;
        };
        writeln!(stdout, "{} {value}", key_style(Cow::Borrowed(label))).ok();
    }

    // APT-Sources: every source of this version.
    if !version.sources.is_empty() {
        write!(stdout, "{}", key_style(Cow::Borrowed("APT-Sources:"))).ok();
        if version.sources.len() == 1 {
            writeln!(
                stdout,
                " {}",
                format_apt_source(&version.sources[0], apt_cfg)
            )
            .ok();
        } else {
            writeln!(stdout).ok();
            for src in &version.sources {
                writeln!(stdout, "  {}", format_apt_source(src, apt_cfg)).ok();
            }
        }
    }

    // APT-Manual-Installed: check dpkg status and auto-installed flag.
    if state.is_installed_of(pkg.name(), pkg.arch()) {
        write!(
            stdout,
            "{}",
            key_style(Cow::Borrowed("APT-Manual-Installed: "))
        )
        .ok();
        if ext_states.is_auto_installed(pkg.name()) {
            writeln!(stdout, "no").ok();
        } else {
            writeln!(stdout, "yes").ok();
        }
    }
}

#[derive(Serialize)]
struct PackageJson<'a> {
    #[serde(flatten)]
    entry: &'a Control,
    #[serde(rename = "APT-Sources")]
    apt_sources: Vec<String>,
    installed: bool,
}

fn display_versions_to_json(
    stdout: &mut impl Write,
    resolved: &ResolvedPackage,
    versions: &[Version],
    state: &AptState,
    apt_cfg: &AptConfig,
) -> Result<(), OutputError> {
    let installed = state.is_installed_of(resolved.key.name(), resolved.key.arch());
    let json_entries: Vec<PackageJson<'_>> = versions
        .iter()
        .map(|v| PackageJson {
            entry: &v.control,
            apt_sources: v
                .sources
                .iter()
                .map(|s| format_apt_source(s, apt_cfg))
                .collect(),
            installed,
        })
        .collect();

    writeln!(stdout, "{}", serde_json::to_string(&json_entries)?).ok();

    Ok(())
}

fn load_state(cfg: &AptConfig) -> Result<(AptState, AptExtendedStates), OutputError> {
    let ext_path = cfg.get_file("Dir::State::extended_states", "var/lib/apt/extended_states");

    let state = AptState::load(cfg).context("Failed to load package state")?;
    let ext_states = AptExtendedStates::from_file_lazy(ext_path);

    Ok((state, ext_states))
}

#[inline]
fn key_style(key: Cow<str>) -> StyledObject<Cow<str>> {
    style(key).bold()
}

/// Format a package's source as an `APT-Sources:` entry, producing
/// `{uri} {description}` like `https://mirror/anthon/debs/ stable/main amd64
/// Packages`. The description comes from the `Acquire::IndexTargets`
/// `Description` template for the Packages index this entry came from (so
/// the rendered shape is configuration, not a hardcoded string); the source
/// itself was resolved from `sources.list` when the database was built. A
/// local `.deb` carries a `file:` base URL and the conventional
/// `local-deb/local-deb` suite/component, so it renders through the same
/// path.
fn format_apt_source(source: &Source, apt_cfg: &AptConfig) -> String {
    // The dpkg status file is not an index: apt names the file itself, e.g.
    // `APT-Sources: /var/lib/dpkg/status`.
    if source.is_dpkg_status() {
        return source.base_url.clone();
    }

    // libapt renders the archive URI without a trailing slash in
    // APT-Sources: its sources.list parser appends one internally
    // (`FixupURI`), but the displayed URI is trimmed, e.g.
    // `http://archive.ubuntu.com/ubuntu unstable/main amd64 Packages`.
    let base_url = source.base_url.trim_end_matches('/').to_string();
    let suite = &source.suite;
    let is_flat = source.component.is_none();

    // Match the `Acquire::IndexTargets` `Description` template against the
    // index path this entry came from (`{component}/binary-{arch}/Packages`,
    // or the bare `Packages` for flat repositories).
    let templates = IndexTargetTemplates::new(apt_cfg);
    let matched_template = if is_flat {
        // Flat repositories have no architecture dimension — pass an empty
        // arch so `flatMetaKey` matches without resolving `APT::Architecture`.
        templates
            .resolve_targets("Packages", suite, &[""], "", "", "", true)
            .ok()
            .and_then(|v| v.into_iter().next())
            .map(|r| (r.description, r.arch))
    } else if let (Some(component), Some(arch)) = (&source.component, &source.arch) {
        let filename = format!("{component}/binary-{arch}/Packages");
        templates
            .resolve_targets(&filename, suite, &[arch], component, "", "", false)
            .ok()
            .and_then(|v| v.into_iter().next())
            .map(|r| (r.description, r.arch))
    } else {
        None
    };

    if let Some((template, arch)) = matched_template {
        let formatted = substitute(
            &template,
            suite,
            source.component.as_deref().unwrap_or(""),
            &arch,
            "",
            "",
        );
        return format!("{base_url} {formatted}");
    }

    // Fallback: no matching IndexTarget (e.g. a file type without a
    // configured target) — degrade to `{uri} {suite}/{component}`.
    match (&source.component, &source.arch) {
        (Some(component), Some(_)) => format!("{base_url} {suite}/{component} Packages"),
        (Some(component), None) => format!("{base_url} {suite}/{component}"),
        (None, _) => format!("{base_url} {suite}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(version: &str, arch: &str, source: Source) -> Version {
        Version {
            control: oma_apt_pkg::deb::parse_control_entry(&format!(
                "Package: vendor-tool\nVersion: {version}\nArchitecture: {arch}\n\n"
            ))
            .unwrap(),
            sources: vec![source],
            arch: oma_apt_pkg::Arch::new(arch),
        }
    }

    #[test]
    fn default_version_is_the_highest_one() {
        let status = Source::dpkg_status("/var/lib/dpkg/status");
        let versions = vec![
            version("1", "amd64", status.clone()),
            version("3", "i386", status),
        ];
        assert_eq!(default_version(&versions).unwrap().version(), Some("3"));
    }

    #[test]
    fn status_source_is_named_by_its_file() {
        let source = Source::dpkg_status("/var/lib/dpkg/status");
        assert_eq!(
            format_apt_source(&source, &AptConfig::new()),
            "/var/lib/dpkg/status"
        );
    }
}
