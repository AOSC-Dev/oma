use clap::{builder::PossibleValue, command, Arg, ArgAction, Command};
use std::{ffi::OsStr, path::PathBuf};

pub fn command_builder() -> Command {
    let dry_run = Arg::new("dry_run")
        .long("dry-run")
        .help("Run Omakase in “dry-run” mode")
        .long_help("Run Omakase in “dry-run” mode. Useful for testing changes and operations without making changes to the system")
        .action(clap::ArgAction::SetTrue);

    let pkgs = Arg::new("packages")
        .num_args(1..)
        .action(clap::ArgAction::Append);

    let no_fixbroken = Arg::new("no_fix_broken")
        .long("no-fix-broken")
        .help("Do not attempt to resolve broken dependencies in the system")
        .action(clap::ArgAction::SetTrue);

    let no_refresh = Arg::new("no_refresh")
        .long("no-refresh")
        .help("Do not refresh repository metadata")
        .action(clap::ArgAction::SetTrue);

    let no_autoremove = Arg::new("no_autoremove")
        .long("no-autoremove")
        .help("Do not remove package(s) without reverse dependencies")
        .action(clap::ArgAction::SetTrue);

    let yes = Arg::new("yes")
        .long("yes")
        .short('y')
        .help("Bypass confirmation prompts")
        .action(clap::ArgAction::SetTrue);

    let force_yes = Arg::new("force_yes")
        .long("force-yes")
        .help("Ignore repository and package dependency issues")
        .action(clap::ArgAction::SetTrue);

    let force_confnew = Arg::new("force_confnew")
        .long("force-confnew")
        .help("Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)")
        .action(clap::ArgAction::SetTrue);

    let dpkg_force_all = Arg::new("dpkg_force_all")
        .long("dpkg-force-all")
        .help("Request dpkg(1) to ignore any issues occurred during the installation and configuration process")
        .action(ArgAction::SetTrue);

    let no_refresh_topics = Arg::new("no_refresh_topics")
        .long("no-refresh-topics")
        .help("Do not refresh topics manifest.json file")
        .action(ArgAction::SetTrue);

    let mut cmd = command!()
        .max_term_width(100)
        .disable_version_flag(true)
        .after_help("本 oma 具有超级小熊猫力")
        .arg(
            Arg::new("debug")
                .long("debug")
                .help("Run oma with debug mode")
                .long_help("Run Omakase with debug output, including details on program parameters and data. Useful for developers and administrators to investigate and report bugs and issues")
                .action(clap::ArgAction::SetTrue)
                .global(true)
        )
        .arg(
            Arg::new("no_color")
                .long("no-color")
                .help("No color output to result")
                .action(ArgAction::SetTrue)
                .global(true)
        )
        .arg(
            Arg::new("no_progress")
            .long("no-progress")
            .help("Do not display progress bar")
            .action(ArgAction::SetTrue)
            .global(true)
        )
        .arg(
            Arg::new("ailurus")
                .long("ailurus")
                .action(ArgAction::Count)
                .hide(true)
        )
        .arg(Arg::new("no_check_dbus").long("no-check-dbus").long_help("Run oma do not check dbus").action(ArgAction::SetTrue).global(true))
        .arg(
            Arg::new("version")
                .long("version")
                .short('v')
                .short_alias('V')
                .action(ArgAction::Version)
                .help("Print version")
        )
        .arg(
            Arg::new("sysroot")
            .long("sysroot")
            .help("Set sysroot target directory")
            .action(ArgAction::Set)
            .global(true)
            .num_args(1)
            .default_value("/")
        )
        .subcommand({
            let mut cmd = Command::new("install")
                .about("Install package(s) from the repository")
                .arg(pkgs.clone().help("Package(s) to install"))
                .arg(
                    Arg::new("install_dbg")
                        .alias("dbg")
                        .long("install-dbg")
                        .help("Install debug symbols for (a) package(s)")
                        .requires("packages")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("reinstall")
                        .long("reinstall")
                        .help("Reinstall package(s) by downloading a current copy from the repository")
                        .requires("packages")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("install_recommends")
                        .long("install-recommends")
                        .help("Install recommended packages(s)")
                        .requires("packages")
                        .conflicts_with("no_install_recommends")
                        .action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("install_suggests")
                        .long("install-suggests")
                        .help("Install suggested package(s)")
                        .requires("packages")
                        .action(ArgAction::SetTrue)
                        .conflicts_with("no_install_suggests")
                )
                .arg(Arg::new("no_install_recommends").long("no-install-recommends").requires("packages").help("Do not install recommend package(s)").conflicts_with("install_recommends").action(ArgAction::SetTrue))
                .arg(Arg::new("no_install_suggests").long("no-install-suggests").requires("packages").help("Do not install recommend package(s)").conflicts_with("install_suggests").action(ArgAction::SetTrue))
                .arg(no_fixbroken.clone().requires("packages"))
                .arg(&no_refresh)
                .arg(yes.clone().requires("packages"))
                .arg(force_yes.clone().requires("packages"))
                .arg(force_confnew.clone().requires("packages"))
                .arg(&dry_run)
                .arg(&dpkg_force_all);

            if cfg!(feature = "aosc") {
                cmd = cmd.arg(&no_refresh_topics);
            }

            cmd
        }
        )
        .subcommand({
            let mut cmd = Command::new("upgrade")
                .alias("dist-upgrade")
                .alias("full-upgrade")
                .about("Upgrade packages installed on the system")
                .arg(pkgs.clone().help("Package(s) to upgrade"))
                .arg(&yes)
                .arg(&force_yes)
                .arg(force_confnew)
                .arg(&dry_run)
                .arg(&dpkg_force_all)
                .arg(&no_refresh_topics);

            if cfg!(feature = "aosc") {
                cmd = cmd.arg(&no_refresh_topics);
            }

            cmd
        }
        )
        .subcommand(
            Command::new("download")
                .about("Download package(s) from the repository")
                .arg(
                    pkgs.clone()
                        .num_args(1..)
                        .required(true)
                        .help("Package(s) to download"),
                )
                .arg(
                    Arg::new("path")
                        .long("path")
                        .short('p')
                        .requires("packages")
                        .action(clap::ArgAction::Set)
                        .help("The path where package(s) should be downloaded to"),
                )
        )
        .subcommand(
            Command::new("remove")
                .alias("delete")
                .alias("autoremove")
                .about("Remove the specified package(s)")
                .arg(pkgs.clone().help("Package(s) to remove"))
                .arg(yes.clone().requires("packages"))
                .arg(force_yes.clone().requires("packages"))
                .arg(no_autoremove.clone().requires("packages"))
                .arg(
                    Arg::new("remove_config")
                        .long("remove-config")
                        .help("Remove package(s) also remove configuration file(s), like apt purge")
                        .action(ArgAction::SetTrue),
                )
                .arg(&dry_run),
        )
        .subcommand(
            Command::new("purge")
                .about("purge (like apt purge) the specified package(s)")
                .hide(true)
                .arg(pkgs.clone().required(true).help("Package(s) to purge"))
                .arg(yes.requires("packages"))
                .arg(force_yes.requires("packages"))
                .arg(no_autoremove.requires("packages"))
                .arg(&dry_run),
        )
        .subcommand({
            let mut cmd = Command::new("refresh")
            .about("Refresh repository metadata/catalog")
            .long_about("Refresh repository metadata/catalog to check for available updates and new packages");

            if cfg!(feature = "aosc") {
                cmd = cmd.arg(&no_refresh_topics);
            }

            cmd
        })
        .subcommand(
            Command::new("show").about("Show information on the specified package(s)").arg(pkgs.clone().required(true)).arg(
                Arg::new("all")
                    .short('a')
                    .long("all")
                    .help("Show information on all available version(s) of (a) package(s) from all repository(ies)")
                    .action(ArgAction::SetTrue)
                    .requires("packages"))
        )
        .subcommand(
            Command::new("search")
                .about("Search for package(s) available from the repository")
                .arg(
                    Arg::new("pattern")
                        .help("Keywords, parts of a path, executable names to search")
                        .action(ArgAction::Set)
                        .num_args(1..)
                        .required(true)
                )
        )
        .subcommand(
            Command::new("files")
                .about("List files in the specified package")
                .arg(
                    Arg::new("package")
                        .help("Package to display a list files of")
                        .action(ArgAction::Set)
                        .num_args(0..=1)
                        .required(true),
                )
                .arg(Arg::new("bin").long("bin").help("Search binary of package(s)").action(ArgAction::SetTrue).requires("package"))
        )
        .subcommand(
            Command::new("provides")
                .about("Search for package(s) that provide(s) certain patterns in a path")
                .arg(
                    Arg::new("pattern")
                        .help("Keywords, parts of a path, executable names to search")
                        .action(ArgAction::Set)
                        .num_args(0..=1)
                        .required(true),
                )
                .arg(Arg::new("bin").long("bin").help("Search binary of package(s)").action(ArgAction::SetTrue).requires("pattern"))
        )
        .subcommand(
            Command::new("fix-broken")
                .about("Resolve broken system dependencies in the system")
                .arg(&dry_run),
        )
        .subcommand({
            let mut cmd = Command::new("pick")
                .about("Install specific version of a package")
                .arg(
                    Arg::new("package")
                        .help("Package to pick specific version for")
                        .action(ArgAction::Set)
                        .num_args(0..=1)
                        .required(true),
                )
                .arg(no_refresh.requires("package"))
                .arg(&dry_run)
                .arg(&dpkg_force_all);

            if cfg!(feature = "aosc") {
                cmd = cmd.arg(&no_refresh_topics);
            }

            cmd
        })
        .subcommand(
            Command::new("mark")
                .about("Mark status for one or multiple package(s)")
                .long_about("Mark status for one or multiple package(s), Omakase will resolve dependencies in accordance with the marked status(es) of the specified package(s)")
                .arg(Arg::new("action").value_parser([
                    PossibleValue::new("hold").help("Lock package version(s), this will prevent the specified package(s) from being updated or downgraded"),
                    PossibleValue::new("unhold").help("Unlock package version(s), this will undo the “hold” status on the specified packge(s)"),
                    PossibleValue::new("manual").help("Mark package(s) as manually installed, this will prevent the specified package(s) from being removed when all reverse dependencies were removed"),
                    PossibleValue::new("auto").help("Mark package(s) as automatically installed, this will mark the specified package(s) for removal when all reverse dependencies were removed")])
                .required(true).num_args(1).action(ArgAction::Set))
                .arg(pkgs.clone().required(true).requires("action").help("Package(s) to mark status for"))
                .arg(&dry_run))
        .subcommand(
            Command::new("command-not-found").hide(true).arg(
                Arg::new("package")
                    .help("Package name")
                    .action(ArgAction::Set)
                    .num_args(0..=1)
                    .required(true),
            ),
        )
        .subcommand(
            Command::new("list")
                .arg(pkgs.clone().help("Package(s) to list"))
                .arg(
                    Arg::new("all")
                        .short('a')
                        .long("all")
                        .help("List all available version(s) of (a) package(s) from all repository(ies)")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("installed")
                        .short('i')
                        .long("installed")
                        .help("List only package(s) currently installed on the system")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("upgradable")
                        .short('u')
                        .long("upgradable")
                        .help("List only package(s) with update(s) available")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("manually-installed")
                        .short('m')
                        .long("manually-installed")
                        .help("List only package(s) with manually installed")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("automatic")
                    .long("automatic")
                    .help("List only package(s) with automatic installed")
                    .action(ArgAction::SetTrue))
                    .about("List package(s) available from the repository"),
        )
        .subcommand(
            Command::new("depends")
                .alias("dep")
                .arg(
                    pkgs.clone()
                        .num_args(1..)
                        .required(true)
                        .help("Package(s) to query dependency(ies) for"),
                )
                .about("Lists dependencies of one or multiple packages"),
        )
        .subcommand(
            Command::new("rdepends")
                .alias("rdep")
                .arg(
                    pkgs.num_args(1..)
                        .required(true)
                        .help("Package(s) to query dependency(ies) for"),
                )
                .about("List reverse dependency(ies) for the specified package(s)"),
        )
        .subcommand(Command::new("clean").about("Clear downloaded package cache"))
        .subcommand(
            Command::new("history")
                        .alias("log")
                        .about("Show a history/log of package changes in the system"))
        .subcommand(
            Command::new("undo")
                        .about("Undo system changes operation"))
        .subcommand(
        Command::new("pkgnames")
                .hide(true)
                .arg(Arg::new("keyword")
                    .action(ArgAction::Set)
                    .num_args(1)
                )
        )
        .subcommands({
            let plugins = list_plugin();
            if let Ok(plugins) = plugins {
                plugins.iter().filter_map(|plugin| {
                    let name = plugin.strip_prefix("oma-");
                    name
                }).map(|name| Command::new(name.to_string())
                .arg(Arg::new("COMMANDS").required(false).num_args(1..).help("Applet specific commands"))
                .about("")).collect()
            } else {
                vec![]
            }
        })
        .subcommand(Command::new("tui").about("Oma tui interface"));

    if cfg!(feature = "aosc") {
        cmd = cmd.subcommand(
            Command::new("topics")
                .alias("topic")
                .about("Manage testing topics enrollment")
                .arg(
                    Arg::new("opt_in")
                        .long("opt-in")
                        .help("Enroll in one or more topic(s), delimited by space")
                        .action(ArgAction::Append)
                        .num_args(1..),
                )
                .arg(
                    Arg::new("opt_out")
                        .long("opt-out")
                        .help("Withdraw from one or more topic(s) and rollback to stable versions, delimited by space")
                        .action(ArgAction::Append)
                        .num_args(1..),
                )
                .arg(&dry_run)
        );
    }

    cmd
}

/// List all the available plugins/helper scripts
fn list_plugin() -> anyhow::Result<Vec<String>> {
    let exe_dir = PathBuf::from("/usr/libexec");
    let plugins_dir = exe_dir.read_dir()?;
    let plugins = plugins_dir
        .filter_map(|x| {
            if let Ok(x) = x {
                let path = x.path();
                let filename = path
                    .file_name()
                    .unwrap_or_else(|| OsStr::new(""))
                    .to_string_lossy();
                if path.is_file() && filename.starts_with("oma-") {
                    return Some(filename.to_string());
                }
            }
            None
        })
        .collect();

    Ok(plugins)
}
