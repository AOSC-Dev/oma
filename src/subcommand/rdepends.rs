use std::borrow::Cow;

use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};

use crate::error::OutputError;

use super::utils::{check_unsupported_stmt, handle_no_result};

pub fn execute(pkgs: Vec<String>, sysroot: String) -> Result<i32, OutputError> {
    for pkg in &pkgs {
        check_unsupported_stmt(pkg);
    }

    let oma_apt_args = OmaAptArgs::builder().sysroot(sysroot.clone()).build();
    let mut apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;

    let (pkgs, no_result) = apt.select_pkg(
        &pkgs.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
        false,
        true,
        false,
    )?;

    handle_no_result(sysroot, no_result)?;

    for pkg in pkgs {
        println!("{}:", pkg.raw_pkg.name());
        println!("  Reverse dependencies:");
        let all_deps = pkg.get_rdeps(&apt.cache)?;

        for (k, v) in all_deps {
            for dep in v.inner() {
                for b_dep in dep {
                    let s = if let (Some(symbol), Some(ver)) = (b_dep.comp_symbol, b_dep.target_ver)
                    {
                        Cow::Owned(format!("({} {symbol} {ver})", pkg.raw_pkg.name()))
                    } else {
                        Cow::Borrowed("")
                    };

                    println!("    {k}: {} {}", b_dep.name, s);
                }
            }
        }
    }

    Ok(0)
}
