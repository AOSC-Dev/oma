use clap::{command, Arg, ArgAction, Command};

pub fn command_builder() -> Command {
    let dry_run = Arg::new("dry_run")
        .short('d')
        .long("dry-run")
        .help("Dry-run oma")
        .action(clap::ArgAction::SetTrue);

    let debug = Arg::new("debug")
        .long("debug")
        .help("Debug mode (with dry-run mode)")
        .action(clap::ArgAction::SetTrue);

    let pkgs = Arg::new("packages")
        .help("Package(s) name")
        .action(clap::ArgAction::Append);

    let no_fixbroken = Arg::new("no_fixbroken")
        .long("no-fixbroken")
        .help("Do not try fix package depends broken status")
        .action(clap::ArgAction::SetTrue);

    let no_upgrade = Arg::new("no_upgrade")
        .long("no-upgrade")
        .help("Do not refresh packages database")
        .action(clap::ArgAction::SetTrue);

    let yes = Arg::new("yes")
        .long("yes")
        .short('y')
        .help("run oma in automatic mode")
        .action(clap::ArgAction::SetTrue);

    let force_yes = Arg::new("force_yes")
        .long("force-yes")
        .help("Force install packages for can't resolve depends")
        .action(clap::ArgAction::SetTrue);

    let force_confnew = Arg::new("force_confnew")
        .long("force-confnew")
        .help("Install package use dpkg --force-confnew")
        .action(clap::ArgAction::SetTrue);

    let dpkg_force_all = Arg::new("dpkg_force_all")
        .long("dpkg-force-all")
        .help("Use dpkg --force-all mode to try fix broken upgrade")
        .action(ArgAction::SetTrue);

    let mark_short_help_gen = |s: &str| format!("Mark package status as {s}");

    command!()
        .arg_required_else_help(true)
        .max_term_width(100)
        .arg(&dry_run)
        .arg(debug)
        .arg(
            Arg::new("ailurus")
                .long("ailurus")
                .action(ArgAction::Count)
                .hide(true),
        )
        .subcommand(
            Command::new("install")
                .about("Install Package")
                .arg(&pkgs)
                .arg(
                    Arg::new("install_dbg")
                        .alias("dbg")
                        .long("install-dbg")
                        .help("Install package(s) debug symbol")
                        .requires("packages")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("reinstall")
                        .long("reinstall")
                        .help("Reinstall package(s)")
                        .requires("packages")
                        .action(ArgAction::SetTrue),
                )
                .arg(no_fixbroken.clone().requires("packages"))
                .arg(no_upgrade.clone().requires("packages"))
                .arg(yes.clone().requires("packages"))
                .arg(force_yes.clone().requires("packages"))
                .arg(force_confnew.clone().requires("packages"))
                .arg(&dry_run)
                .arg(&dpkg_force_all),
        )
        .subcommand(
            Command::new("upgrade")
                .alias("dist-upgrade")
                .alias("full-upgrade")
                .about("Update Package")
                .arg(&pkgs)
                .arg(&yes)
                .arg(&force_yes)
                .arg(force_confnew)
                .arg(&dry_run)
                .arg(&dpkg_force_all),
        )
        .subcommand(
            Command::new("download")
                .about("Download Package")
                .arg(pkgs.clone().num_args(1..).required(true))
                .arg(
                    Arg::new("path")
                        .long("path")
                        .short('p')
                        .requires("path")
                        .action(clap::ArgAction::Set),
                ),
        )
        .subcommand(
            Command::new("remove")
                .alias("delete")
                .alias("purge")
                .about("Delete Package")
                .arg(pkgs.clone().num_args(1..).required(true))
                .arg(yes.requires("packages"))
                .arg(force_yes.requires("packages"))
                .arg(
                    Arg::new("keep_config")
                        .long("keep-config")
                        .short('k')
                        .help("Keep package config")
                        .action(ArgAction::SetTrue),
                )
                .arg(&dry_run),
        )
        .subcommand(Command::new("refresh").about("efresh Package database"))
        .subcommand(
            Command::new("show").about("Show package").arg(&pkgs).arg(
                Arg::new("all")
                    .short('a')
                    .help("Query all package")
                    .action(ArgAction::SetTrue),
            ),
        )
        .subcommand(
            Command::new("search")
                .about("Search Packages")
                .arg(pkgs.clone().num_args(1..).required(true)),
        )
        .subcommand(
            Command::new("list-files")
                .about("Query package list files")
                .arg(
                    Arg::new("package")
                        .help("Pakcage name")
                        .action(ArgAction::Set)
                        .num_args(0..=1)
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("provides")
                .about("Search file from packages")
                .arg(
                    Arg::new("package")
                        .help("Pakcage name")
                        .action(ArgAction::Set)
                        .num_args(0..=1)
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("fix-broken")
                .about("Fix system dependencies broken status")
                .arg(&dry_run),
        )
        .subcommand(
            Command::new("pick")
                .about("Pick a package version")
                .arg(
                    Arg::new("package")
                        .help("Pakcage name")
                        .action(ArgAction::Set)
                        .num_args(0..=1)
                        .required(true),
                )
                .arg(no_fixbroken.requires("package"))
                .arg(no_upgrade.requires("package"))
                .arg(&dry_run)
                .arg(&dpkg_force_all),
        )
        .subcommand(
            Command::new("mark")
                .about("Mark a package status")
                .subcommand(
                    Command::new("hold")
                        .arg(pkgs.clone().num_args(1..).required(true))
                        .about(mark_short_help_gen("hold")),
                )
                .subcommand(
                    Command::new("unhold")
                        .arg(pkgs.clone().num_args(1..).required(true))
                        .about(mark_short_help_gen("unhold")),
                )
                .subcommand(
                    Command::new("manual")
                        .arg(pkgs.clone().num_args(1..).required(true))
                        .about(mark_short_help_gen("manual")),
                )
                .subcommand(
                    Command::new("auto")
                        .arg(pkgs.clone().num_args(1..).required(true))
                        .about(mark_short_help_gen("auto")),
                )
                .arg(&dry_run),
        )
        .subcommand(
            Command::new("command-not-found").hide(true).arg(
                Arg::new("package")
                    .help("Pakcage name")
                    .action(ArgAction::Set)
                    .num_args(0..=1)
                    .required(true),
            ),
        )
        .subcommand(
            Command::new("list")
                .arg(&pkgs)
                .arg(
                    Arg::new("all")
                        .short('a')
                        .help("Query all packages")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("installed")
                        .short('i')
                        .help("Query installed packages")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("upgradable")
                        .short('u')
                        .help("Query upgradable packages")
                        .action(ArgAction::SetTrue),
                )
                .about("List of packages"),
        )
        .subcommand(
            Command::new("depends")
                .alias("dep")
                .arg(pkgs.clone().num_args(1..).required(true))
                .about("Query package dependencies"),
        )
        .subcommand(
            Command::new("rdepends")
                .alias("rdep")
                .arg(pkgs.num_args(1..).required(true))
                .about("Query package reverse dependencies"),
        )
        .subcommand(Command::new("clean").about("Clean downloaded packages archive"))
        .subcommand(
            Command::new("history")
                .alias("log")
                .about("See oma history"),
        )
}
