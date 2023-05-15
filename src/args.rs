use anstyle::RgbColor;
use clap::{
    builder::{PossibleValue, Styles},
    command, Arg, ArgAction, Command,
};

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

    let mut cmd = command!()
        .styles(
            Styles::styled()
            .usage(
                anstyle::Style::new()
                    .bold()
                    .fg_color(Some(anstyle::Color::Rgb(RgbColor(147, 190, 242))))
                    .underline()
            )
            .header(
                anstyle::Style::new()
                    .bold()
                    .fg_color(Some(anstyle::Color::Rgb(RgbColor(147, 190, 242))))
                    .underline()
            )
            .invalid(
                anstyle::Style::new()
                    .bold()
                    .fg_color(Some(anstyle::Color::Rgb(RgbColor(240, 243, 235))))
            )
            .literal(anstyle::Style::new().bold().fg_color(Some(anstyle::Color::Rgb(RgbColor(102, 186, 183)))))
        )
        .arg_required_else_help(true)
        .max_term_width(100)
        .disable_version_flag(true)
        .arg(
            Arg::new("debug")
                .long("debug")
                .help("Run oma with debug mode")
                .long_help("Run Omakase with debug output, including details on program parameters and data. Useful for developers and administrators to investigate and report bugs and issues")
                .action(clap::ArgAction::SetTrue)
                .global(true)
        )
        .arg(
            Arg::new("ailurus")
                .long("ailurus")
                .action(ArgAction::Count)
                .hide(true)
        )
        .arg(
            Arg::new("version")
                .long("version")
                .short('v')
                .short_alias('V')
                .action(ArgAction::Version)
                .help("Print version")
        )
        .subcommand(
            Command::new("install")
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
                        .action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("install_suggests")
                        .long("install-suggests")
                        .help("Install suggested package(s)")
                        .requires("packages")
                        .action(ArgAction::SetTrue)
                )
                .arg(Arg::new("no_install_recommends").long("no-install-recommends").requires("packages").help("Do not install recommend package(s)").conflicts_with("install_recommends").action(ArgAction::SetTrue))
                .arg(Arg::new("no_install_suggests").long("no-install-suggests").requires("packages").help("Do not install recommend package(s)").conflicts_with("install_suggests").action(ArgAction::SetTrue))
                .arg(no_fixbroken.clone().requires("packages"))
                .arg(&no_refresh)
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
                .about("Upgrade packages installed on the system")
                .arg(&no_autoremove)
                .arg(pkgs.clone().help("Package(s) to upgrade"))
                .arg(&yes)
                .arg(&force_yes)
                .arg(force_confnew)
                .arg(&dry_run)
                .arg(&dpkg_force_all),
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
                        .requires("path")
                        .action(clap::ArgAction::Set)
                        .help("The path where package(s) should be downloaded to"),
                ),
        )
        .subcommand(
            Command::new("remove")
                .alias("delete")
                .alias("purge")
                .about("Remove the specified package(s)")
                .arg(pkgs.clone().required(true).help("Package(s) to remove"))
                .arg(yes.requires("packages"))
                .arg(force_yes.requires("packages"))
                .arg(no_autoremove.requires("packages"))
                .arg(
                    Arg::new("keep_config")
                        .long("keep-config")
                        .short('k')
                        .help("Keep configuration file(s), this may be useful if you intend to install the package(s) again in the future")
                        .action(ArgAction::SetTrue),
                )
                .arg(&dry_run),
        )
        .subcommand(Command::new("refresh")
            .about("Refresh repository metadata/catalog")
            .long_about("Refresh repository metadata/catalog to check for available updates and new packages"))
        .subcommand(
            Command::new("show").about("Show information on the specified package(s)").arg(pkgs.clone().required(true)).arg(
                Arg::new("all")
                    .short('a')
                    .help("Show information on all available version(s) of (a) package(s) from all repository(ies)")
                    .action(ArgAction::SetTrue)
                    .requires("packages")
            ),
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
            Command::new("list-files")
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
        .subcommand(
            Command::new("pick")
                .about("Install specific version of a package")
                .arg(
                    Arg::new("package")
                        .help("Package to pick specific version for")
                        .action(ArgAction::Set)
                        .num_args(0..=1)
                        .required(true),
                )
                .arg(no_fixbroken.requires("package"))
                .arg(no_refresh.requires("package"))
                .arg(&dry_run)
                .arg(&dpkg_force_all),
        )
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
                        .help("List all available version(s) of (a) package(s) from all repository(ies)")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("installed")
                        .short('i')
                        .help("List only package(s) currently installed on the system")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("upgradable")
                        .short('u')
                        .help("List only package(s) with update(s) available")
                        .action(ArgAction::SetTrue),
                )
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
                .about("Show a history/log of package changes in the system"),
        )
        .subcommand(
        Command::new("pkgnames")
                .hide(true)
                .arg(Arg::new("keyword")
                    .action(ArgAction::Set)
                    .num_args(1)
                )
        );

    if cfg!(feature = "aosc") {
        cmd = cmd.subcommand(
            Command::new("topics")
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
                ),
        );
    }

    cmd
}
