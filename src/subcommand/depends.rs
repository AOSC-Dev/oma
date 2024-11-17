use std::{borrow::Cow, io::stdout, io::Write};

use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::PackagesMatcher,
};
use oma_utils::dpkg::dpkg_arch;

use crate::error::OutputError;

use super::utils::{check_unsupported_stmt, handle_no_result};

pub fn execute(
    pkgs: Vec<String>,
    sysroot: String,
    json: bool,
    another_apt_options: Vec<String>,
    no_progress: bool,
) -> Result<i32, OutputError> {
    for pkg in &pkgs {
        check_unsupported_stmt(pkg);
    }

    let apt_config = AptConfig::new();
    let oma_apt_args = OmaAptArgs::builder()
        .sysroot(sysroot.clone())
        .another_apt_options(another_apt_options)
        .build();
    let apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

    let arch = dpkg_arch(&sysroot)?;
    let matcher = PackagesMatcher::builder()
        .cache(&apt.cache)
        .filter_candidate(true)
        .filter_downloadable_candidate(false)
        .select_dbg(false)
        .native_arch(&arch)
        .build();

    let (pkgs, no_result) = matcher.match_pkgs(pkgs.iter().map(|x| x.as_str()))?;

    handle_no_result(sysroot, no_result, no_progress)?;

    if !json {
        for pkg in pkgs {
            println!("{}:", pkg.raw_pkg.fullname(true));
            let all_deps = pkg.get_deps(&apt.cache)?;

            for (k, v) in all_deps {
                for dep in v.inner() {
                    for b_dep in dep {
                        let s = if let Some(comp_ver) = b_dep.comp_ver {
                            Cow::Owned(format!("({comp_ver})"))
                        } else {
                            Cow::Borrowed("")
                        };

                        println!("  {k}: {} {}", b_dep.name, s);
                    }
                }
            }
        }
    } else {
        let mut stdout = stdout();
        for pkg in pkgs {
            writeln!(
                stdout,
                "{}",
                serde_json::json!({
                    "name": pkg.raw_pkg.fullname(true),
                    "deps": pkg.get_deps(&apt.cache)?,
                })
            )
            .ok();
        }
    }

    Ok(0)
}
