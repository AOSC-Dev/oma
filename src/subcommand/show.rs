use std::io::Write;
use std::str::FromStr;
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
    QueryGroup,
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
        let (apt_db, dpkg, ext_states) = load_apt_db_and_dpkg(apt_cfg)?;

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

        for (i, group) in resolution.groups.iter().enumerate() {
            match group {
                QueryGroup::Db(entries) => display_group(
                    &mut stdout,
                    entries,
                    &dpkg,
                    &ext_states,
                    all,
                    json,
                    &source_lookup,
                    apt_cfg,
                )?,
                QueryGroup::Local(entry) => {
                    let entries = [EntryWithSource {
                        entry: entry.as_ref(),
                        source: None,
                    }];
                    display_group(
                        &mut stdout,
                        &entries,
                        &dpkg,
                        &ext_states,
                        all,
                        json,
                        &source_lookup,
                        apt_cfg,
                    )?;
                }
            }

            if i != resolution.groups.len() - 1 {
                writeln!(stdout).ok();
            }

            // Show additional version hint for single package without --all
            let entry_count = match group {
                QueryGroup::Db(entries) => entries.len(),
                QueryGroup::Local(_) => 1,
            };
            if !all && !json && resolution.groups.len() == 1 && entry_count > 1 {
                info!("{}", fl!("additional-version", len = entry_count));
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
    let shown_entries: Box<dyn Iterator<Item = &EntryWithSource<'_>>> = if show_all {
        Box::new(entries.iter())
    } else {
        // Show only the entry with the highest version
        Box::new(
            entries
                .iter()
                .max_by(|a, b| {
                    let a_ver = Version::from_str(a.entry.version.as_deref().unwrap_or("0")).ok();
                    let b_ver = Version::from_str(b.entry.version.as_deref().unwrap_or("0")).ok();
                    a_ver.cmp(&b_ver)
                })
                .into_iter(),
        )
    };

    for (idx, entry) in shown_entries.enumerate() {
        if show_all && idx != 0 {
            writeln!(stdout).ok();
        }

        display_single_entry(
            stdout,
            entry.entry,
            entry.source,
            dpkg,
            ext_states,
            source_lookup,
            apt_cfg,
        );
    }
}

fn display_single_entry(
    stdout: &mut impl Write,
    entry: &PackageEntry,
    source: Option<&str>,
    dpkg: &DpkgState,
    ext_states: &AptExtendedStates,
    source_lookup: &SourceLookup,
    apt_cfg: &AptConfig,
) {
    for (label, field) in DISPLAY_FIELDS {
        let value: Option<Cow<'_, str>> = match *field {
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
        };

        let Some(value) = value else {
            continue;
        };

        writeln!(stdout, "{} {value}", key_style(Cow::Borrowed(label))).ok();
    }

    // APT-Sources: decode the APT lists filename back to the original format
    if let Some(src) = source {
        let formatted = format_apt_source(src, source_lookup, apt_cfg);
        write!(stdout, "{}", key_style(Cow::Borrowed("APT-Sources:"))).ok();
        writeln!(stdout, " {formatted}").ok();
    }

    // APT-Manual-Installed: check dpkg status and auto-installed flag
    if entry.is_installed(dpkg) {
        write!(
            stdout,
            "{}",
            key_style(Cow::Borrowed("APT-Manual-Installed: "))
        )
        .ok();
        if entry.is_auto_installed(dpkg, ext_states) {
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
            entry: ews.entry,
            apt_sources: ews.source,
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
    cfg: &AptConfig,
) -> Result<(AptDb, DpkgState, AptExtendedStates), OutputError> {
    let lists_dir = cfg.get_dir("Dir::State::lists", "var/lib/apt/lists");
    let dpkg_path = cfg.get_file("Dir::State::status", "var/lib/dpkg/status");
    let ext_path = cfg.get_file("Dir::State::extended_states", "var/lib/apt/extended_states");
    let apt_cache = crate::utils::get_apt_cache_path("Dir::Cache::oma-aptdb", "oma-aptdb.bincode");

    let apt_db =
        AptDb::load_or_build(&apt_cache, &lists_dir).context("Failed to load apt database")?;

    let dpkg = DpkgState::from_file(&dpkg_path).context("Failed to load dpkg status")?;

    let ext_states =
        AptExtendedStates::from_file(ext_path).context("Failed to read apt extended states")?;

    Ok((apt_db, dpkg, ext_states))
}

#[inline]
fn key_style(key: Cow<str>) -> StyledObject<Cow<str>> {
    style(key).bold()
}

/// Decode an APT lists filename stem and format as APT-Sources.
///
/// The result is `"{source_url} {substituted_description}"`, matching
/// APT's `$(SITE) + Description` format (see `debmetaindex.cc`).
fn format_apt_source(source: &str, source_lookup: &SourceLookup, apt_cfg: &AptConfig) -> String {
    let cvt = AptListFilename::new();
    let Ok(decoded) = cvt.decode(source) else {
        return source.to_string();
    };

    let Some(matched) = source_lookup.resolve(&decoded) else {
        return decoded;
    };

    let templates = IndexTargetTemplates::new(apt_cfg);
    let base_url = matched.entry.url();
    let is_flat = matched.component.is_none();
    let suite = &matched.entry.suite;

    let matched_template = if is_flat {
        templates
            .resolve_targets(
                matched.filename,
                suite,
                &["$(ARCHITECTURE)"],
                "",
                "",
                "",
                true,
            )
            .ok()
            .and_then(|v| v.into_iter().next())
            .map(|r| (r.description, r.arch))
    } else if let Some(component) = matched.component {
        let archs: Vec<&str> = matched
            .entry
            .archs
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();

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

    if let Some(component) = matched.component {
        if matched.filename.is_empty() {
            format!("{base_url} {}/{}", suite, component)
        } else {
            format!("{base_url} {}/{} {}", suite, component, matched.filename)
        }
    } else if is_flat {
        format!("{base_url} {} {}", suite, matched.filename)
    } else {
        format!("{base_url} {}", suite)
    }
}
