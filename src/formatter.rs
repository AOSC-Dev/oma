use anyhow::{bail, Context, Result};
use std::{format, io::Write, sync::atomic::Ordering};

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
    fl,
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
            warn!("{}", fl!("download-not-done"));
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
    pub fn new(
        config: Config,
        yes: bool,
        force_yes: bool,
        dpkg_force_confnew: bool,
        dpkg_force_all: bool,
    ) -> Self {
        if yes {
            rust_apt::raw::config::raw::config_set(
                "APT::Get::Assume-Yes".to_owned(),
                "true".to_owned(),
            );
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
            warn!("{}", fl!("force-auto-mode"));
            config.set("APT::Get::force-yes", "true");
            tracing::debug!("APT::Get::force-Yes is set to true");
        }

        if dpkg_force_all {
            warn!("{}", fl!("dpkg-force-all-mode"));
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
                        unmet_dependency: fl!("dep-does-not-exist", name = d.name.as_str()),
                        specified_dependency: format!("{} {}", ver.parent().name(), ver.version()),
                    })
                }

                if let Some(dep_pkg) = dep_pkg {
                    if dep_pkg.candidate().is_none() {
                        v.push(UnmetTable {
                            package: style(&d.name).red().bold().to_string(),
                            unmet_dependency: fl!("dep-does-not-exist", name = d.name.as_str()),
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
                        unmet_dependency: fl!("dep-does-not-exist", name = d.name.as_str()),
                        specified_dependency: format!("{} {}", ver.parent().name(), ver.version()),
                    })
                }

                if let Some(dep_pkg) = dep_pkg {
                    if dep_pkg.candidate().is_none() {
                        v.push(UnmetTable {
                            package: style(&d.name).red().bold().to_string(),
                            unmet_dependency: fl!("dep-does-not-exist", name = d.name.as_str()),
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
                "{} {}{}\n",
                fl!("count-pkg-has-desc", count = v.len()),
                style(fl!("unmet-dep")).red().bold(),
                fl!("semicolon")
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
    let changes = cache.get_changes(true)?;

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
            "{} {}{}\n",
            fl!("count-pkg-has-desc", count = v.len()),
            style(fl!("unmet-dep")).red().bold(),
            fl!("semicolon")
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
    writeln!(out, "{:<80}\n", style(fl!("dep-error")).on_red().bold())?;
    writeln!(out, "{}\n", fl!("dep-error-desc"),)?;
    writeln!(out, "{}\n", fl!("contact-admin-tips"))?;
    writeln!(out, "    {}", style(fl!("how-to-abort")).bold())?;
    writeln!(out, "    {}\n\n", style(fl!("how-to-op-with-x")).bold())?;

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
pub fn display_result(action: &Action, cache: &Cache, no_pager: bool) -> Result<Vec<InstallRow>> {
    if DRYRUN.load(Ordering::Relaxed) {
        return Ok(vec![]);
    }

    let mut pager = Pager::new(no_pager, true)?;
    let pager_name = pager.pager_name().to_owned();
    let mut out = pager.get_writer()?;

    write_review_help_message(&mut out, pager_name).ok();

    if pager_name == Some("less") {
        let has_x11 = std::env::var("DISPLAY");

        let line1 = format!("    {}", fl!("end-review"));
        let line2 = format!("    {}", fl!("cc-to-abort"));

        if has_x11.is_ok() {
            let line3 = format!("    {}\n\n", fl!("how-to-op-with-x"));

            writeln!(out, "{}", style(line1).bold()).ok();
            writeln!(out, "{}", style(line2).bold()).ok();
            writeln!(out, "{}", style(line3).bold()).ok();
        } else {
            let line3 = format!("    {}\n\n", fl!("how-to-op"));

            writeln!(out, "{}", style(line1).bold()).ok();
            writeln!(out, "{}", style(line2).bold()).ok();
            writeln!(out, "{}", style(line3).bold()).ok();
        }
    }

    let list = result_inner(action, &pager)?;

    writeln!(
        out,
        "{}{}",
        style(fl!("total-download-size")).bold(),
        HumanBytes(download_size(&list, cache)?)
    )
    .ok();

    let (symbol, abs_install_size_change) = match cache.depcache().disk_size() {
        DiskSpace::Require(n) => ("+", n),
        DiskSpace::Free(n) => ("-", n),
    };

    writeln!(
        out,
        "{}{}{}",
        style(fl!("change-storage-usage")).bold(),
        symbol,
        HumanBytes(abs_install_size_change)
    )
    .ok();

    drop(out);
    let exited = pager.wait_for_exit()?;

    if exited {
        Ok(list)
    } else {
        bail!("")
    }
}

pub fn result_inner(action: &Action, pager: &Pager) -> Result<Vec<InstallRow>> {
    let mut out = pager.get_writer()?;

    let update = action.update.clone();
    let install = action.install.clone();
    let del = action.del.clone();
    let reinstall = action.reinstall.clone();
    let downgrade = action.downgrade.clone();

    if !del.is_empty() {
        writeln!(
            out,
            "{} {}{}\n",
            fl!("count-pkg-has-desc", count = del.len()),
            style(fl!("removed")).red().bold(),
            fl!("semicolon")
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
            "{} {}{}\n",
            fl!("count-pkg-has-desc", count = install.len()),
            style(fl!("installed")).green().bold(),
            fl!("semicolon")
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
            "{} {}{}\n",
            fl!("count-pkg-has-desc", count = update.len()),
            style(fl!("upgrade")).color256(87),
            fl!("colon")
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
            "{} {}{}\n",
            fl!("count-pkg-has-desc", count = downgrade.len()),
            style(fl!("downgraded")).yellow().bold(),
            fl!("colon")
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
            "{} {}{}\n",
            fl!("count-pkg-has-desc", count = reinstall.len()),
            style(fl!("reinstall")).blue().bold(),
            fl!("colon")
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

    Ok(list)
}

/// Get download size
pub fn download_size(install_and_update: &[InstallRow], cache: &Cache) -> Result<u64> {
    let mut result = 0;

    for i in install_and_update {
        let pkg = cache.get(&i.name_no_color).context(fl!(
            "can-not-get-pkg-from-database",
            name = i.name_no_color.as_str()
        ))?;

        let ver = pkg.get_version(&i.new_version).context(fl!(
            "can-not-get-pkg-version-from-database",
            name = i.name_no_color.as_str(),
            version = i.new_version.as_str()
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
            style(fl!("pending-op")).bold().bg(Color::Color256(25))
        )?;
    }

    writeln!(w)?;
    writeln!(w, "{}\n", fl!("review-msg"))?;
    writeln!(
        w,
        "{}\n",
        fl!(
            "oma-may",
            a = style(fl!("install")).green().to_string(),
            b = style(fl!("remove")).red().to_string(),
            c = style(fl!("upgrade")).color256(87).to_string(),
            d = style(fl!("downgrade")).yellow().to_string(),
            e = style(fl!("reinstall")).blue().to_string()
        ),
    )?;
    writeln!(w)?;

    Ok(())
}

pub fn capitalize_str(mut v: String) -> String {
    v.get_mut(0..1).map(|s| {
        s.make_ascii_uppercase();
        &*s
    });

    v
}
