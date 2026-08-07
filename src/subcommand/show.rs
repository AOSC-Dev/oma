use std::collections::BTreeMap;
use std::io::Write;
use std::str::FromStr;
use std::sync::Arc;
use std::{borrow::Cow, io::stdout};

use anyhow::Context;
use clap::Args;
use clap_complete::ArgValueCompleter;
use debversion::Version;
use dialoguer::console::{StyledObject, style};
use oma_apt_pkg::apt_sources::SourceLookup;
use oma_apt_pkg::apt_sources::{IndexTargetTemplates, substitute};
use oma_apt_pkg::{
    AptConfig, AptDb, AptExtendedStates, AptListFilename, DpkgState, EntryWithSource, PackageEntry,
};
use oma_console::indicatif::HumanBytes;
use serde::Serialize;
use spdlog::info;

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
        let source_lookup = SourceLookup::build(apt_cfg);
        let (mut apt_db, dpkg, ext_states) = load_apt_db_and_dpkg(apt_cfg)?;

        // Resolve each query: local `.deb` files are parsed directly,
        // everything else is matched against the package database.
        let resolution = apt_db
            .resolve_queries(packages)
            .context("Failed to resolve package queries")?;

        handle_no_result(
            resolution.no_match.iter().map(String::as_str).collect(),
            config.no_progress(),
        )?;

        let mut stdout = stdout();

        for (i, entries) in resolution.groups.iter().enumerate() {
            display_group(
                &mut stdout,
                entries,
                &dpkg,
                &ext_states,
                all,
                json,
                &source_lookup,
                apt_cfg,
            )?;

            if i != resolution.groups.len() - 1 {
                writeln!(stdout).ok();
            }

            // Show additional version hint for single package without --all
            let entry_count = entries.len();
            if !all && !json && resolution.groups.len() == 1 && entry_count > 1 {
                info!(
                    "{}",
                    fl!("additional-version", len = entry_count.saturating_sub(1))
                );
            }
        }

        Ok(ExitHandle::default())
    }
}

/// Display one resolved group, honoring the JSON flag.
#[allow(clippy::too_many_arguments)]
fn display_group(
    stdout: &mut impl Write,
    entries: &[EntryWithSource<'_>],
    dpkg: &DpkgState,
    ext_states: &AptExtendedStates,
    all: bool,
    json: bool,
    source_lookup: &SourceLookup,
    apt_cfg: &AptConfig,
) -> Result<(), OutputError> {
    if json {
        display_entries_to_json(stdout, entries, dpkg)?;
    } else {
        display_entries(
            stdout,
            entries,
            dpkg,
            ext_states,
            all,
            source_lookup,
            apt_cfg,
        );
    }
    Ok(())
}

fn display_entries(
    stdout: &mut impl Write,
    entries: &[EntryWithSource<'_>],
    dpkg: &DpkgState,
    ext_states: &AptExtendedStates,
    show_all: bool,
    source_lookup: &SourceLookup,
    apt_cfg: &AptConfig,
) {
    // Group entries by version so the same version coming from multiple
    // sources (e.g. a repo package and a local `.deb`) renders as one block
    // listing every source.
    let mut versions: BTreeMap<&str, Vec<&EntryWithSource<'_>>> = BTreeMap::new();
    for entry in entries {
        versions
            .entry(entry.entry.version.as_deref().unwrap_or("0"))
            .or_default()
            .push(entry);
    }

    let shown = if show_all {
        versions.iter().collect::<Vec<_>>()
    } else {
        // Show only the highest version.
        versions
            .iter()
            .max_by(|(a, _), (b, _)| {
                let a_ver = Version::from_str(a).ok();
                let b_ver = Version::from_str(b).ok();
                a_ver.cmp(&b_ver)
            })
            .into_iter()
            .collect::<Vec<_>>()
    };

    for (idx, (_, group)) in shown.iter().enumerate() {
        if show_all && idx != 0 {
            writeln!(stdout).ok();
        }

        display_version_group(stdout, group, dpkg, ext_states, source_lookup, apt_cfg);
    }
}

/// Resolve one display field from a package entry, in the order of
/// [`DISPLAY_FIELDS`].
fn field_value<'a>(entry: &'a PackageEntry, field: &str) -> Option<Cow<'a, str>> {
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

/// Display all entries sharing one version as a single block, merging fields
/// (the first entry carrying each field wins, so a repo entry's
/// `Download-Size` shows next to a local `.deb`) and listing every source.
fn display_version_group(
    stdout: &mut impl Write,
    group: &[&EntryWithSource<'_>],
    dpkg: &DpkgState,
    ext_states: &AptExtendedStates,
    source_lookup: &SourceLookup,
    apt_cfg: &AptConfig,
) {
    for (label, field) in DISPLAY_FIELDS {
        let Some(value) = group
            .iter()
            .find_map(|e| field_value(e.entry.as_ref(), field))
        else {
            continue;
        };
        writeln!(stdout, "{} {value}", key_style(Cow::Borrowed(label))).ok();
    }

    // APT-Sources: every source of this version, deduplicated.
    let mut sources: Vec<&str> = Vec::new();
    for e in group {
        if let Some(src) = e.source.as_deref()
            && !sources.contains(&src)
        {
            sources.push(src);
        }
    }
    if !sources.is_empty() {
        write!(stdout, "{}", key_style(Cow::Borrowed("APT-Sources:"))).ok();
        if sources.len() == 1 {
            writeln!(
                stdout,
                " {}",
                format_apt_source(sources[0], source_lookup, apt_cfg)
            )
            .ok();
        } else {
            writeln!(stdout).ok();
            for src in &sources {
                writeln!(
                    stdout,
                    "  {}",
                    format_apt_source(src, source_lookup, apt_cfg)
                )
                .ok();
            }
        }
    }

    // APT-Manual-Installed: check dpkg status and auto-installed flag.
    let primary = &group[0].entry;
    if primary.is_installed(dpkg) {
        write!(
            stdout,
            "{}",
            key_style(Cow::Borrowed("APT-Manual-Installed: "))
        )
        .ok();
        if primary.is_auto_installed(dpkg, ext_states) {
            writeln!(stdout, "no").ok();
        } else {
            writeln!(stdout, "yes").ok();
        }
    }
}

#[derive(Serialize)]
struct PackageJson<'a> {
    #[serde(flatten)]
    entry: &'a PackageEntry,
    #[serde(rename = "APT-Sources")]
    apt_sources: Option<&'a str>,
    installed: bool,
}

fn display_entries_to_json(
    stdout: &mut impl Write,
    entries: &[EntryWithSource<'_>],
    dpkg: &DpkgState,
) -> Result<(), OutputError> {
    let json_entries: Vec<PackageJson<'_>> = entries
        .iter()
        .map(|ews| PackageJson {
            entry: ews.entry.as_ref(),
            apt_sources: ews.source.as_deref(),
            installed: ews.entry.is_installed(dpkg),
        })
        .collect();

    writeln!(
        stdout,
        "{}",
        serde_json::to_string(&json_entries).map_err(|e| OutputError {
            description: e.to_string(),
            source: None,
        })?
    )
    .ok();

    Ok(())
}

fn load_apt_db_and_dpkg(
    cfg: &Arc<AptConfig>,
) -> Result<(AptDb, DpkgState, AptExtendedStates), OutputError> {
    let lists_dir = cfg.get_dir("Dir::State::lists", "var/lib/apt/lists");
    let dpkg_path = cfg.get_file("Dir::State::status", "var/lib/dpkg/status");
    let ext_path = cfg.get_file("Dir::State::extended_states", "var/lib/apt/extended_states");
    let apt_cache = crate::utils::get_apt_cache_path("Dir::Cache::oma-aptdb", "oma-aptdb.bincode");

    let apt_db =
        AptDb::load_or_build(cfg, &apt_cache, &lists_dir).context("Failed to load apt database")?;

    let dpkg = DpkgState::from_file(&dpkg_path).context("Failed to load dpkg status")?;

    let ext_states =
        AptExtendedStates::from_file(ext_path).context("Failed to read apt extended states")?;

    Ok((apt_db, dpkg, ext_states))
}

#[inline]
fn key_style(key: Cow<str>) -> StyledObject<Cow<str>> {
    style(key).bold()
}

/// Decode an APT lists filename stem and format as an `APT-Sources:` entry
/// using the `Acquire::IndexTargets` `Description` template, producing
/// `{uri} {description}` like `https://mirror/anthon/debs/ stable/main amd64
/// Packages`.
fn format_apt_source(source: &str, source_lookup: &SourceLookup, apt_cfg: &AptConfig) -> String {
    // A `file:` source is a local `.deb`, stored as its `file:` URI. Render
    // it in the same `{uri} {suite}/{component}` shape the template produces
    // for repository sources, with APT's conventional `local-deb/local-deb`
    // suite/component for local debs.
    if let Some(uri) = source.strip_prefix("file:") {
        return format!("file:{uri} local-deb/local-deb");
    }

    let cvt = AptListFilename::new();
    let Ok(decoded) = cvt.decode(source) else {
        return source.to_string();
    };

    let Some(matched) = source_lookup.resolve(&decoded) else {
        return decoded;
    };

    // `archive_uri` keeps a trailing slash, like libapt's URI handling.
    let base_url = format!("{}/", matched.entry.url());
    let suite = matched.entry.suite.trim_end_matches('/');
    let is_flat = matched.component.is_none();

    let templates = IndexTargetTemplates::new(apt_cfg);

    let matched_template = if is_flat {
        // Flat repositories have no architecture dimension — pass an empty
        // arch so `flatMetaKey` matches without resolving `APT::Architecture`.
        templates
            .resolve_targets(matched.filename, suite, &[""], "", "", "", true)
            .ok()
            .and_then(|v| v.into_iter().next())
            .map(|r| (r.description, r.arch))
    } else if let Some(component) = matched.component {
        // Architectures to try: the source's declared ones, falling back to
        // the one in the index path (`binary-<arch>`).
        let archs: Vec<&str> = matched
            .entry
            .archs
            .as_ref()
            .filter(|a| !a.is_empty())
            .map(|a| a.iter().map(String::as_str).collect())
            .unwrap_or_else(|| index_arch(&decoded));

        templates
            .resolve_targets(matched.filename, suite, &archs, component, "", "", false)
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
            matched.component.unwrap_or(""),
            &arch,
            "",
            "",
        );
        return format!("{base_url} {formatted}");
    }

    // Fallback: no matching IndexTarget (e.g. a file type without a
    // configured target) — degrade to `{uri} {suite}/{component} {type}`.
    let bare = matched.filename.rsplit('/').next().unwrap_or("");
    if let Some(component) = matched.component {
        if bare.is_empty() {
            format!("{base_url} {suite}/{component}")
        } else {
            format!("{base_url} {suite}/{component} {bare}")
        }
    } else if is_flat {
        format!("{base_url} {suite} {bare}")
    } else {
        format!("{base_url} {suite}")
    }
}

/// The architecture from an index path segment `binary-<arch>`, if present.
fn index_arch(decoded: &str) -> Vec<&str> {
    decoded
        .rsplit('/')
        .find(|seg| seg.starts_with("binary-"))
        .and_then(|seg| seg.strip_prefix("binary-"))
        .into_iter()
        .collect()
}
