use anyhow::{bail, Context, Result};
use std::{io::Write, sync::atomic::Ordering};

use console::{style, Color};
use indicatif::HumanBytes;
use rust_apt::{
    cache::Cache,
    config::Config,
    package::{DepType, Dependency, Package, Version},
    raw::progress::{AcquireProgress, InstallProgress},
    util::{
        cmp_versions, get_apt_progress_string, terminal_height, terminal_width, time_str, unit_str,
        DiskSpace, NumSys,
    },
};
use tabled::{
    settings::{
        object::{Columns, Segment},
        Alignment, Format, Modify, Style,
    },
    Table, Tabled,
};

use std::cmp::Ordering as CmpOrdering;

use crate::{
    oma::{Action, InstallRow},
    pager::Pager,
    pkg::OmaDependency,
    warn, ALLOWCTRLC, DRYRUN,
};

// TODO: Make better structs for pkgAcquire items, workers, owners.
/// AptAcquireProgress is the default struct for the update method on the cache.
///
/// This struct mimics the output of `apt update`.
#[derive(Default, Debug)]
pub struct NoProgress {
    _lastline: usize,
    _pulse_interval: usize,
    _disable: bool,
}

impl NoProgress {
    /// Returns a new default progress instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the AptAcquireProgress in a box
    /// To easily pass through for progress
    pub fn new_box() -> Box<dyn AcquireProgress> {
        Box::new(Self::new())
    }
}

/// Do not output anything apt AcquireProgress
impl AcquireProgress for NoProgress {
    fn pulse_interval(&self) -> usize {
        0
    }

    fn hit(&mut self, _id: u32, description: String) {
        tracing::debug!("{}", description);
    }

    fn fetch(&mut self, _id: u32, description: String, _file_size: u64) {
        tracing::debug!("{}", description);
    }

    fn fail(&mut self, _id: u32, description: String, _status: u32, _error_text: String) {
        tracing::debug!("{}", description);
    }

    fn pulse(
        &mut self,
        _workers: Vec<rust_apt::raw::progress::Worker>,
        _percent: f32,
        _total_bytes: u64,
        _current_bytes: u64,
        _current_cps: u64,
    ) {
    }

    fn done(&mut self) {}

    fn start(&mut self) {}

    fn stop(
        &mut self,
        fetched_bytes: u64,
        elapsed_time: u64,
        current_cps: u64,
        _pending_errors: bool,
    ) {
        if fetched_bytes != 0 {
            warn!("Download is not done, running apt download ...\nIf you are not install package from local sources/local packages, Please run debug mode and report to upstream: https://github.com/aosc-dev/oma");
            tracing::debug!(
                "Fetched {} in {} ({}/s)",
                unit_str(fetched_bytes, NumSys::Decimal),
                time_str(elapsed_time),
                unit_str(current_cps, NumSys::Decimal)
            );
        }
    }
}

pub struct OmaAptInstallProgress {
    config: Config,
}

impl OmaAptInstallProgress {
    #[allow(dead_code)]
    pub fn new(config: Config, yes: bool, force_yes: bool, dpkg_force_confnew: bool, dpkg_force_all: bool) -> Self {
        if yes {
            rust_apt::raw::config::raw::config_set("APT::Get::Assume-Yes".to_owned(), "true".to_owned());
            tracing::debug!("APT::Get::Assume-Yes is set to true");
        }

        if dpkg_force_confnew {
            config.set("Dpkg::Options::", "--force-confnew");
            tracing::debug!("Dpkg::Options:: is set to --force-confnew");
        } else if yes {
            config.set("Dpkg::Options::", "--force-confold");
            tracing::debug!("Dpkg::Options:: is set to --force-confold");
        }

        if force_yes {
            warn!("Now you are using FORCE automatic mode, if this is not your intention, press Ctrl + C to stop the operation!!!!!");
            config.set("APT::Get::force-yes", "true");
            tracing::debug!("APT::Get::force-Yes is set to true");
        }

        if dpkg_force_all {
            warn!("Now you are using DPKG FORCE ALL mode, if this is not your intention, press Ctrl + C to stop the operation!!!!!");
            config.set("Dpkg::Options::", "--force-all");
            tracing::debug!("Dpkg::Options:: is set to --force-all");
        }

        Self { config }
    }

    /// Return the AptInstallProgress in a box
    /// To easily pass through to do_install
    pub fn new_box(
        config: Config,
        yes: bool,
        force_yes: bool,
        dpkg_force_confnew: bool,
        dpkg_force_all: bool,
    ) -> Box<dyn InstallProgress> {
        Box::new(Self::new(
            config,
            yes,
            force_yes,
            dpkg_force_confnew,
            dpkg_force_all,
        ))
    }
}

impl InstallProgress for OmaAptInstallProgress {
    fn status_changed(
        &mut self,
        _pkgname: String,
        steps_done: u64,
        total_steps: u64,
        _action: String,
    ) {
        // Get the terminal's width and height.
        let term_height = terminal_height();
        let term_width = terminal_width();

        // Save the current cursor position.
        print!("\x1b7");

        // Go to the progress reporting line.
        print!("\x1b[{term_height};0f");
        std::io::stdout().flush().unwrap();

        // Convert the float to a percentage string.
        let percent = steps_done as f32 / total_steps as f32;
        let mut percent_str = (percent * 100.0).round().to_string();

        let percent_padding = match percent_str.len() {
            1 => "  ",
            2 => " ",
            3 => "",
            _ => unreachable!(),
        };

        percent_str = percent_padding.to_owned() + &percent_str;

        // Get colors for progress reporting.
        // NOTE: The APT implementation confusingly has 'Progress-fg' for 'bg_color',
        // and the same the other way around.
        let bg_color = self
            .config
            .find("Dpkg::Progress-Fancy::Progress-fg", "\x1b[42m");
        let fg_color = self
            .config
            .find("Dpkg::Progress-Fancy::Progress-bg", "\x1b[30m");
        const BG_COLOR_RESET: &str = "\x1b[49m";
        const FG_COLOR_RESET: &str = "\x1b[39m";

        print!("{bg_color}{fg_color}Progress: [{percent_str}%]{BG_COLOR_RESET}{FG_COLOR_RESET} ");

        // The length of "Progress: [100%] ".
        const PROGRESS_STR_LEN: usize = 17;

        // Print the progress bar.
        // We should safely be able to convert the `usize`.try_into() into the `u32`
        // needed by `get_apt_progress_string`, as usize ints only take up 8 bytes on a
        // 64-bit processor.
        print!(
            "{}",
            get_apt_progress_string(percent, (term_width - PROGRESS_STR_LEN).try_into().unwrap())
        );
        std::io::stdout().flush().unwrap();

        // If this is the last change, remove the progress reporting bar.
        // if steps_done == total_steps {
        // print!("{}", " ".repeat(term_width));
        // print!("\x1b[0;{}r", term_height);
        // }
        // Finally, go back to the previous cursor position.
        print!("\x1b8");
        std::io::stdout().flush().unwrap();
    }

    // TODO: Need to figure out when to use this.
    fn error(&mut self, _pkgname: String, _steps_done: u64, _total_steps: u64, _error: String) {}
}

#[derive(Tabled)]
pub struct UnmetTable {
    #[tabled(rename = "Package")]
    pub package: String,
    #[tabled(rename = "Unmet Dependency")]
    pub unmet_dependency: String,
    #[tabled(rename = "Specified Dependency")]
    pub specified_dependency: String,
}

pub fn find_unmet_deps_with_markinstall(
    cache: &Cache,
    ver: &Version,
    use_page: bool,
) -> Result<bool> {
    let dep = ver.get_depends(&DepType::Depends);
    let pdep = ver.get_depends(&DepType::PreDepends);

    let mut v = vec![];

    if let Some(dep) = dep {
        let dep = OmaDependency::map_deps(dep);
        for b_dep in dep {
            for d in b_dep {
                let dep_pkg = cache.get(&d.name);
                if dep_pkg.is_none() {
                    v.push(UnmetTable {
                        package: style(&d.name).red().bold().to_string(),
                        unmet_dependency: format!("Dep: {} does not exist", d.name),
                        specified_dependency: format!("{} {}", ver.parent().name(), ver.version()),
                    })
                }

                if let Some(dep_pkg) = dep_pkg {
                    if dep_pkg.candidate().is_none() {
                        v.push(UnmetTable {
                            package: style(&d.name).red().bold().to_string(),
                            unmet_dependency: format!("Dep: {} does not exist", d.name),
                            specified_dependency: format!(
                                "{} {}",
                                ver.parent().name(),
                                ver.version()
                            ),
                        })
                    }
                }
            }
        }
    }

    if let Some(pdep) = pdep {
        let dep = OmaDependency::map_deps(pdep);
        for b_dep in dep {
            for d in b_dep {
                let dep_pkg = cache.get(&d.name);
                if dep_pkg.is_none() {
                    v.push(UnmetTable {
                        package: style(&d.name).red().bold().to_string(),
                        unmet_dependency: format!("Dep: {} does not exist", d.name),
                        specified_dependency: format!("{} {}", ver.parent().name(), ver.version()),
                    })
                }

                if let Some(dep_pkg) = dep_pkg {
                    if dep_pkg.candidate().is_none() {
                        v.push(UnmetTable {
                            package: style(&d.name).red().bold().to_string(),
                            unmet_dependency: format!("Dep: {} does not exist", d.name),
                            specified_dependency: format!(
                                "{} {}",
                                ver.parent().name(),
                                ver.version()
                            ),
                        })
                    }
                }
            }
        }
    }

    if !v.is_empty() {
        if use_page {
            let mut pager = Pager::new(false, false)?;
            let mut out = pager.get_writer()?;

            let mut table = Table::new(&v);

            table
                .with(Modify::new(Segment::all()).with(Alignment::left()))
                .with(Modify::new(Columns::new(2..3)).with(Alignment::left()))
                .with(Style::psql())
                .with(Modify::new(Segment::all()).with(Format::content(|s| format!(" {s} "))));

            write_dep_issue_msg(&mut out).ok();

            writeln!(
                out,
                "{} package(s) has {}\n",
                v.len(),
                style("unmet dependencies:").red().bold()
            )
            .ok();

            writeln!(out, "{table}").ok();

            drop(out);
            pager.wait_for_exit().ok();
        } else {
            for i in &v {
                warn!(
                    "Pkg {} {}",
                    i.specified_dependency,
                    i.unmet_dependency.to_ascii_lowercase()
                );
            }
        }
    }

    Ok(!v.is_empty())
}

pub fn find_unmet_deps(cache: &Cache) -> Result<bool> {
    let changes = cache.get_changes(true);

    let mut v = vec![];

    for c in changes {
        if let Some(cand) = c.candidate() {
            let rdep = c.rdepends_map();
            let rdep_dep = rdep.get(&DepType::Depends);
            let rdep_predep = rdep.get(&DepType::PreDepends);
            let rdep_breaks = rdep.get(&DepType::Breaks);
            let rdep_conflicts = rdep.get(&DepType::Conflicts);

            // Format dep
            if let Some(rdep_dep) = rdep_dep {
                format_deps(rdep_dep, cache, &cand, &mut v, &c);
            }

            // Format predep
            if let Some(rdep_predep) = rdep_predep {
                format_deps(rdep_predep, cache, &cand, &mut v, &c);
            }

            // Format breaks
            if let Some(rdep_breaks) = rdep_breaks {
                format_breaks(rdep_breaks, cache, &mut v, &c, &cand, "Breaks");
            }

            // Format Conflicts
            if let Some(rdep_conflicts) = rdep_conflicts {
                format_breaks(rdep_conflicts, cache, &mut v, &c, &cand, "Conflicts");
            }
        }
    }

    if !v.is_empty() {
        let mut pager = Pager::new(false, false)?;
        let mut out = pager.get_writer()?;

        let mut table = Table::new(&v);

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            .with(Modify::new(Columns::new(2..3)).with(Alignment::left()))
            .with(Style::psql())
            .with(Modify::new(Segment::all()).with(Format::content(|s| format!(" {s} "))));

        write_dep_issue_msg(&mut out)?;

        writeln!(
            out,
            "{} package(s) has {}\n",
            v.len(),
            style("unmet dependencies:").red().bold()
        )
        .ok();

        writeln!(out, "{table}").ok();

        drop(out);
        pager.wait_for_exit().ok();
    }

    Ok(!v.is_empty())
}

pub fn write_dep_issue_msg(out: &mut dyn Write) -> Result<()> {
    ALLOWCTRLC.store(true, Ordering::Relaxed);
    writeln!(out, "{:<80}\n", style("Dependency Error").on_red().bold())?;
    writeln!(out, "Omakase has detected dependency errors(s) in your system and cannot proceed with\nyour specified operation(s). This may be caused by missing or mismatched\npackages, or that you have specified a version of a package that is not\ncompatible with your system.\n")?;
    writeln!(
        out,
        "Please contact your system administrator or developer\n"
    )?;
    writeln!(
        out,
        "    {}",
        style("Press [q] or [Ctrl-c] to abort.").bold()
    )?;
    writeln!(
        out,
        "    {}\n\n",
        style("Press [PgUp/Dn], arrow keys, or use the mouse wheel to scroll.").bold()
    )?;

    Ok(())
}

fn format_deps(
    rdep: &[Dependency],
    cache: &Cache,
    cand: &Version,
    v: &mut Vec<UnmetTable>,
    c: &Package,
) {
    let rdep = OmaDependency::map_deps(rdep);
    for b_rdep in rdep {
        for dep in b_rdep {
            let pkg = cache.get(&dep.name);
            if let Some(pkg) = pkg {
                if pkg.is_installed() {
                    let comp = dep.comp_symbol;
                    let ver = dep.ver;
                    if let (Some(comp), Some(need_ver)) = (comp, ver) {
                        match comp.as_str() {
                            ">=" => {
                                // 1: 2.36-4   2: 2.36-2
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 >= 2.36-4，但用户在安装 2.36-2
                                if cmp == CmpOrdering::Greater {
                                    v.push(UnmetTable {
                                        package: style(dep.name).red().bold().to_string(),
                                        unmet_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            dep.comp_ver.unwrap()
                                        ),
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            ">>" => {
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 >> 2.36-4，但用户在安装 2.36-2
                                if cmp != CmpOrdering::Less {
                                    v.push(UnmetTable {
                                        package: style(dep.name).red().bold().to_string(),
                                        unmet_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            dep.comp_ver.unwrap()
                                        ),
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            ">" => {
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 > 2.36-4，但用户在安装 2.36-2
                                if cmp != CmpOrdering::Less {
                                    v.push(UnmetTable {
                                        package: style(dep.name).red().bold().to_string(),
                                        unmet_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            dep.comp_ver.unwrap()
                                        ),
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "=" => {
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 = 2.36-4，但用户在安装 2.36-2
                                if cmp != CmpOrdering::Equal {
                                    v.push(UnmetTable {
                                        package: style(dep.name).red().bold().to_string(),
                                        unmet_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            dep.comp_ver.unwrap()
                                        ),
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "<=" => {
                                // 1: 2.36-4 2: 2.36-6
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 <= 2.36-4，但用户在安装 2.36-6
                                if cmp == CmpOrdering::Less {
                                    v.push(UnmetTable {
                                        package: style(dep.name).red().bold().to_string(),
                                        unmet_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            dep.comp_ver.unwrap()
                                        ),
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "<<" => {
                                // 1: 2.36-4 2: 2.36-6
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 <= 2.36-4，但用户在安装 2.36-6
                                if cmp != CmpOrdering::Greater {
                                    v.push(UnmetTable {
                                        package: style(dep.name).red().bold().to_string(),
                                        unmet_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            dep.comp_ver.unwrap()
                                        ),
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "<" => {
                                // 1: 2.36-4 2: 2.36-6
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 <= 2.36-4，但用户在安装 2.36-6
                                if cmp != CmpOrdering::Greater {
                                    v.push(UnmetTable {
                                        package: style(dep.name).red().bold().to_string(),
                                        unmet_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            dep.comp_ver.unwrap()
                                        ),
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            x => panic!("Unsupport symbol: {x}, pkg: {}", dep.name),
                        }
                    }
                }
            }
        }
    }
}

fn format_breaks(
    rdep_breaks: &[Dependency],
    cache: &Cache,
    v: &mut Vec<UnmetTable>,
    c: &Package,
    cand: &Version,
    typ: &str,
) {
    let rdep = OmaDependency::map_deps(rdep_breaks);
    for b_rdep in rdep {
        for dep in b_rdep {
            let dep_pkg = cache.get(&dep.name);
            if let Some(dep_pkg) = dep_pkg {
                if dep.comp_ver.is_none() {
                    if dep_pkg.is_installed() {
                        v.push(UnmetTable {
                            package: style(dep.name).red().bold().to_string(),
                            unmet_dependency: format!("{typ}: {}", dep_pkg.name()),
                            specified_dependency: format!("{} {}", c.name(), cand.version()),
                        })
                    }
                } else if dep_pkg.is_installed() {
                    if let (Some(comp), Some(break_ver)) = (dep.comp_symbol, dep.ver) {
                        match comp.as_str() {
                            ">=" => {
                                // a: breaks b >= 1.0，满足要求的条件是 break_ver > cand.version
                                let cmp = cmp_versions(&break_ver, cand.version());
                                if cmp != CmpOrdering::Greater {
                                    v.push(UnmetTable {
                                        package: style(dep.name).red().bold().to_string(),
                                        unmet_dependency: format!(
                                            "{typ}: {} {}",
                                            dep_pkg.name(),
                                            dep.comp_ver.unwrap()
                                        ),
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            ">>" => {
                                // a: breaks b >> 1.0，满足要求的条件是 break_ver >>= cand.version
                                let cmp = cmp_versions(&break_ver, cand.version());
                                if cmp == CmpOrdering::Less {
                                    v.push(UnmetTable {
                                        package: style(dep.name).red().bold().to_string(),
                                        unmet_dependency: format!(
                                            "{typ}: {} {}",
                                            dep_pkg.name(),
                                            dep.comp_ver.unwrap()
                                        ),
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            ">" => {
                                // a: breaks b > 1.0，满足要求的条件是 break_ver >= cand.version
                                let cmp = cmp_versions(&break_ver, cand.version());
                                if cmp == CmpOrdering::Less {
                                    v.push(UnmetTable {
                                        package: style(dep.name).red().bold().to_string(),
                                        unmet_dependency: format!(
                                            "{typ}: {} {}",
                                            dep_pkg.name(),
                                            dep.comp_ver.unwrap()
                                        ),
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "<=" => {
                                // a: breaks b <= 1.0，满足要求的条件是 break_ver < cand.version
                                let cmp = cmp_versions(&break_ver, cand.version());
                                if cmp != CmpOrdering::Less {
                                    v.push(UnmetTable {
                                        package: style(dep.name).red().bold().to_string(),
                                        unmet_dependency: format!(
                                            "{typ}: {} {}",
                                            dep_pkg.name(),
                                            dep.comp_ver.unwrap()
                                        ),
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "<<" => {
                                // a: breaks b << 1.0，满足要求的条件是 break_ver <= cand.version
                                let cmp = cmp_versions(&break_ver, cand.version());
                                if cmp == CmpOrdering::Greater {
                                    v.push(UnmetTable {
                                        package: style(dep.name).red().bold().to_string(),
                                        unmet_dependency: format!(
                                            "{typ}: {} {}",
                                            dep_pkg.name(),
                                            dep.comp_ver.unwrap()
                                        ),
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "<" => {
                                // a: breaks b << 1.0，满足要求的条件是 break_ver <= cand.version
                                let cmp = cmp_versions(&break_ver, cand.version());
                                if cmp == CmpOrdering::Greater {
                                    v.push(UnmetTable {
                                        package: style(dep.name).red().bold().to_string(),
                                        unmet_dependency: format!(
                                            "{typ}: {} {}",
                                            dep_pkg.name(),
                                            dep.comp_ver.unwrap()
                                        ),
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            x => panic!("Unsupport symbol: {x}, pkg: {}", dep.name),
                        }
                    }
                }
            }
        }
    }
}

/// Display apt resolver results
pub fn display_result(action: &Action, cache: &Cache, no_pager: bool) -> Result<()> {
    if DRYRUN.load(Ordering::Relaxed) {
        return Ok(());
    }

    let update = action.update.clone();
    let install = action.install.clone();
    let del = action.del.clone();
    let reinstall = action.reinstall.clone();
    let downgrade = action.downgrade.clone();

    let mut pager = Pager::new(no_pager, true)?;
    let pager_name = pager.pager_name().to_owned();
    let mut out = pager.get_writer()?;

    write_review_help_message(&mut out, pager_name).ok();

    if pager_name == Some("less") {
        let has_x11 = std::env::var("DISPLAY");

        if has_x11.is_ok() {
            let line1 = "    Press [q] to end review";
            let line2 = "    Press [Ctrl-c] to abort";
            let line3 = "    Press [PgUp/Dn], arrow keys, or use the mouse wheel to scroll.\n\n";

            writeln!(out, "{}", style(line1).bold()).ok();
            writeln!(out, "{}", style(line2).bold()).ok();
            writeln!(out, "{}", style(line3).bold()).ok();
        } else {
            let line1 = "    Press [q] to end review";
            let line2 = "    Press [Ctrl-c] to abort";
            let line3 = "    Press [PgUp/Dn] or arrow keys to scroll.\n\n";

            writeln!(out, "{}", style(line1).bold()).ok();
            writeln!(out, "{}", style(line2).bold()).ok();
            writeln!(out, "{}", style(line3).bold()).ok();
        }
    }

    if !del.is_empty() {
        writeln!(
            out,
            "{} packages will be {}:\n",
            del.len(),
            style("REMOVED").red().bold()
        )
        .ok();

        let mut table = Table::new(del);

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            .with(Modify::new(Columns::new(1..2)).with(Alignment::left()))
            .with(Style::psql());

        writeln!(out, "{table}\n\n").ok();
    }

    if !install.is_empty() {
        writeln!(
            out,
            "{} packages will be {}:\n",
            install.len(),
            style("installed").green().bold()
        )?;

        let mut table = Table::new(&install);

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            // Install Size column should align right
            .with(Modify::new(Columns::new(2..3)).with(Alignment::right()))
            .with(Style::psql())
            .with(Modify::new(Segment::all()).with(Format::content(|s| format!(" {s} "))));

        writeln!(out, "{table}\n\n").ok();
    }

    if !update.is_empty() {
        writeln!(
            out,
            "{} packages will be {}:\n",
            update.len(),
            style("upgraded").color256(87)
        )?;

        let mut table = Table::new(&update);

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            // Install Size column should align right
            .with(Modify::new(Columns::new(2..3)).with(Alignment::right()))
            .with(Style::psql())
            .with(Modify::new(Segment::all()).with(Format::content(|s| format!(" {s} "))));

        writeln!(out, "{table}\n\n").ok();
    }

    if !downgrade.is_empty() {
        writeln!(
            out,
            "{} packages will be {}:\n",
            downgrade.len(),
            style("downgraded").yellow().bold()
        )?;

        let mut table = Table::new(&downgrade);

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            // Install Size column should align right
            .with(Modify::new(Columns::new(1..2)).with(Alignment::right()))
            .with(Style::psql())
            .with(Modify::new(Segment::all()).with(Format::content(|s| format!(" {s} "))));

        writeln!(out, "{table}\n\n").ok();
    }

    if !reinstall.is_empty() {
        writeln!(
            out,
            "{} packages will be {}:\n",
            reinstall.len(),
            style("reinstall").blue().bold()
        )?;

        let mut table = Table::new(&reinstall);

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            // Install Size column should align right
            .with(Modify::new(Columns::new(2..3)).with(Alignment::right()))
            .with(Style::psql())
            .with(Modify::new(Segment::all()).with(Format::content(|s| format!(" {s} "))));

        writeln!(out, "{table}\n\n").ok();
    }

    let mut list = vec![];
    list.extend(install);
    list.extend(update);
    list.extend(downgrade);

    writeln!(
        out,
        "{} {}",
        style("Total download size:").bold(),
        HumanBytes(download_size(&list, cache)?)
    )
    .ok();

    let (symbol, abs_install_size_change) = match cache.depcache().disk_size() {
        DiskSpace::Require(n) => ("+", n),
        DiskSpace::Free(n) => ("-", n),
    };

    writeln!(
        out,
        "{} {}{}",
        style("Estimated change in storage usage:").bold(),
        symbol,
        HumanBytes(abs_install_size_change)
    )
    .ok();

    writeln!(out)?;

    drop(out);
    let success = pager.wait_for_exit()?;

    if success {
        Ok(())
    } else {
        // User aborted the operation
        bail!("")
    }
}

/// Get download size
pub fn download_size(install_and_update: &[InstallRow], cache: &Cache) -> Result<u64> {
    let mut result = 0;

    for i in install_and_update {
        let pkg = cache
            .get(&i.name_no_color)
            .context(format!("Can not get package {}", i.name_no_color))?;

        let ver = pkg.get_version(&i.new_version).context(format!(
            "Can not get package {} version {}",
            i.name_no_color, i.new_version
        ))?;

        let size = ver.size();

        result += size;
    }

    Ok(result)
}

fn write_review_help_message(w: &mut dyn Write, pager_name: Option<&str>) -> Result<()> {
    if pager_name == Some("less") {
        writeln!(
            w,
            "{:<80}",
            style("Pending Operations").bold().bg(Color::Color256(25))
        )?;
    }

    writeln!(w)?;
    writeln!(w, "Shown below is an overview of the pending changes Omakase will apply to your\nsystem, please review them carefully.\n")?;
    writeln!(
        w,
        "Omakase may {}, {}, {}, {}, or {} packages in order\nto fulfill your requested changes.",
        style("install").green(),
        style("remove").red(),
        style("upgrade").color256(87),
        style("downgrade").yellow(),
        style("reinstall").blue()
    )?;
    writeln!(w)?;

    Ok(())
}
