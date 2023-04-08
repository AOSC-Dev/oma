use anyhow::{Context, Result};
use clap::{ArgMatches, Command};
use console::{style, Term};
use os_release::OsRelease;
use std::{io::Write, process::exit, sync::atomic::Ordering};
use tracing::metadata::LevelFilter;
use tracing_subscriber::{
    fmt, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, Layer,
};

use crate::{args::command_builder, oma::Oma, AILURUS, DRYRUN};

const PREFIX_LEN: u16 = 10;

pub enum OmaCommand {
    /// Install Package
    Install(InstallOptions),
    Upgrade(UpgradeOptions),
    /// Download Package
    Download(Download),
    /// Delete Package
    Remove(RemoveOptions),
    /// Refresh Package database
    Refresh,
    /// Show Package
    Show(Show),
    /// Search Package
    Search(Search),
    /// package list files
    ListFiles(ListFiles),
    /// Search file from package
    Provides(Provides),
    /// Fix system dependencies broken status
    FixBroken(FixBroken),
    /// Pick a package version
    Pick(PickOptions),
    /// Mark a package status
    Mark(Mark),
    CommandNotFound(CommandNotFound),
    /// List of packages
    List(ListOptions),
    /// Check package dependencies
    Depends(Dep),
    /// Check package reverse dependencies
    Rdepends(Dep),
    /// Clean downloaded packages
    Clean,
    /// See omakase log
    History,
}

pub struct Dep {
    /// Package(s) name
    pub pkgs: Vec<String>,
}

pub struct CommandNotFound {
    pub kw: String,
}

pub struct FixBroken {
    /// Dry-run oma
    pub dry_run: bool,
    pub dpkg_force_all: bool,
}
pub struct Download {
    /// Package(s) name
    pub packages: Vec<String>,
    /// Download to path
    pub path: Option<String>,
}

pub struct InstallOptions {
    /// Package(s) name
    pub packages: Option<Vec<String>>,
    /// Install package(s) debug symbol
    pub install_dbg: bool,
    /// Reinstall package(s)
    pub reinstall: bool,
    /// Do not try fix package depends broken status
    pub no_fixbroken: bool,
    /// Do not refresh packages database
    pub no_refresh: bool,
    /// Automatic run oma install
    pub yes: bool,
    /// Force install packages for can't resolve depends
    pub force_yes: bool,
    /// Install package use dpkg --force-confnew
    pub force_confnew: bool,
    /// Dry-run oma
    pub dry_run: bool,
    pub dpkg_force_all: bool,
}

pub struct UpgradeOptions {
    /// Package(s) name
    pub packages: Option<Vec<String>>,
    /// Automatic run oma install
    pub yes: bool,
    /// Force install packages for can't resolve depends
    pub force_yes: bool,
    /// Install package use dpkg --force-confnew
    pub force_confnew: bool,
    /// Dry-run oma
    pub dry_run: bool,
    pub dpkg_force_all: bool,
    pub no_autoremove: bool,
}

pub struct ListFiles {
    /// Package name
    pub package: String,
}

pub struct PickOptions {
    /// Package name
    pub package: String,
    /// Do not try fix package depends broken status
    pub no_fixbroken: bool,
    /// Do not refresh packages database
    pub no_upgrade: bool,
    /// Dry-run oma
    pub dry_run: bool,
}

pub struct Provides {
    /// Search keyword
    pub kw: String,
}

pub struct RemoveOptions {
    /// Package(s) name
    pub packages: Vec<String>,
    /// Automatic run oma install
    pub yes: bool,
    /// Force install packages for can't resolve depends
    pub force_yes: bool,
    /// Keep package config
    pub keep_config: bool,
    /// Dry-run oma
    pub dry_run: bool,
    pub no_autoremove: bool,
}

pub struct Show {
    /// Package(s) name
    pub packages: Vec<String>,
    pub is_all: bool,
}

pub struct Search {
    /// Search keyword(s)
    pub keyword: Vec<String>,
}

pub struct Mark {
    pub action: MarkAction,
    /// Dry-run oma
    pub dry_run: bool,
}

pub enum MarkAction {
    /// Hold package version
    Hold(MarkActionArgs),
    /// Unhold package version
    Unhold(MarkActionArgs),
    /// Set package status to manual install
    Manual(MarkActionArgs),
    /// Set package status to auto install
    Auto(MarkActionArgs),
}

pub struct MarkActionArgs {
    pub pkgs: Vec<String>,
}

pub struct ListOptions {
    pub packages: Option<Vec<String>>,
    pub all: bool,
    pub installed: bool,
    pub upgradable: bool,
}

pub struct OmaCommandRunner {
    pub cmd: Command,
}

impl OmaCommandRunner {
    pub fn new() -> Self {
        Self {
            cmd: command_builder(),
        }
    }
}

pub trait CommandMatcher {
    fn match_cmd(&self) -> Result<OmaCommand>;
    fn run(&self) -> Result<()> {
        match self.match_cmd()? {
            OmaCommand::Install(v) => Oma::build_async_runtime()?.install(v),
            OmaCommand::Upgrade(v) => Oma::build_async_runtime()?.update(v),
            OmaCommand::Download(v) => Oma::build_async_runtime()?.download(v),
            OmaCommand::Remove(v) => Oma::remove(v),
            OmaCommand::Refresh => Oma::build_async_runtime()?.refresh(),
            OmaCommand::Show(v) => Oma::show(&v.packages, v.is_all),
            OmaCommand::Search(v) => Oma::search(&v.keyword.join(" ")),
            OmaCommand::ListFiles(v) => Oma::list_files(&v.package),
            OmaCommand::Provides(v) => Oma::search_file(&v.kw),
            OmaCommand::FixBroken(v) => Oma::build_async_runtime()?.fix_broken(v),
            OmaCommand::Pick(v) => Oma::build_async_runtime()?.pick(v),
            OmaCommand::Mark(v) => Oma::mark(v),
            OmaCommand::CommandNotFound(v) => Oma::command_not_found(&v.kw),
            OmaCommand::List(v) => Oma::list(&v),
            OmaCommand::Depends(v) => Oma::dep(&v.pkgs, false),
            OmaCommand::Rdepends(v) => Oma::dep(&v.pkgs, true),
            OmaCommand::Clean => Oma::clean(),
            OmaCommand::History => Oma::log(),
        }
    }
}

impl CommandMatcher for OmaCommandRunner {
    fn match_cmd(&self) -> Result<OmaCommand> {
        let matches = self.cmd.clone();
        let matches = &matches.get_matches();

        if matches.get_count("ailurus") == 3 {
            AILURUS.store(true, Ordering::Relaxed);
        } else if matches.get_count("ailurus") != 0 {
            println!(
                "{} unexpected argument '{}' found\n",
                style("error:").red().bold(),
                style("\x1b[33m--ailurus\x1b[0m").bold()
            );
            println!("{}: oma <COMMAND>\n", style("Usage").bold().underlined());
            println!("For more information, try '{}'.", style("--help").bold());
            exit(3);
        }

        if let Some((_, args)) = matches.subcommand() {
            if let Ok(v) = args.try_get_one::<bool>("dry_run") {
                if v == Some(&true) {
                    DRYRUN.store(true, Ordering::Relaxed);
                    tracing_subscriber::registry()
                        .with(
                            fmt::layer()
                                .with_writer(std::io::stdout)
                                .without_time()
                                .with_target(false)
                                .with_filter(if matches.get_flag("debug") {
                                    LevelFilter::DEBUG
                                } else {
                                    LevelFilter::INFO
                                }),
                        )
                        .try_init()
                        .expect("Can not setup dry_run logger");

                    tracing::info!("Running in Dry-run mode");
                }
            }
        } else if matches.get_flag("debug") {
            tracing_subscriber::registry()
                .with(
                    fmt::layer()
                        .with_writer(std::io::stdout)
                        .without_time()
                        .with_target(false)
                        .with_filter(LevelFilter::DEBUG),
                )
                .try_init()?;
        }

        tracing::debug!(
            "oma version: {}\n OS: {:#?}",
            env!("CARGO_PKG_VERSION"),
            OsRelease::new()
        );

        let pkgs_getter = |args: &ArgMatches| {
            args.get_many::<String>("packages")
                .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>())
        };

        let m = match matches.subcommand() {
            Some(("install", args)) => OmaCommand::Install(InstallOptions {
                packages: pkgs_getter(args),
                install_dbg: args.get_flag("install_dbg"),
                reinstall: args.get_flag("reinstall"),
                no_fixbroken: args.get_flag("no_fix_broken"),
                no_refresh: args.get_flag("no_refresh"),
                yes: args.get_flag("yes"),
                force_yes: args.get_flag("force_yes"),
                force_confnew: args.get_flag("force_confnew"),
                dry_run: args.get_flag("dry_run"),
                dpkg_force_all: args.get_flag("dpkg_force_all"),
            }),
            Some(("upgrade", args)) => OmaCommand::Upgrade(UpgradeOptions {
                packages: pkgs_getter(args),
                yes: args.get_flag("yes"),
                force_yes: args.get_flag("force_yes"),
                force_confnew: args.get_flag("force_confnew"),
                dry_run: args.get_flag("dry_run"),
                dpkg_force_all: args.get_flag("dpkg_force_all"),
                no_autoremove: args.get_flag("no_autoremove"),
            }),
            Some(("download", args)) => OmaCommand::Download(Download {
                packages: pkgs_getter(args).unwrap(),
                path: args.get_one::<String>("path").cloned(),
            }),
            Some(("remove", args)) => OmaCommand::Remove(RemoveOptions {
                packages: pkgs_getter(args).unwrap(),
                yes: args.get_flag("yes"),
                force_yes: args.get_flag("force_yes"),
                keep_config: args.get_flag("keep_config"),
                dry_run: args.get_flag("dry_run"),
                no_autoremove: args.get_flag("no_autoremove"),
            }),
            Some(("refresh", _)) => OmaCommand::Refresh,
            Some(("show", args)) => OmaCommand::Show(Show {
                packages: pkgs_getter(args).unwrap(),
                is_all: args.get_flag("all"),
            }),
            Some(("search", args)) => OmaCommand::Search(Search {
                keyword: args
                    .get_many::<String>("pattern")
                    .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>())
                    .unwrap(),
            }),
            Some(("list-files", args)) => OmaCommand::ListFiles(ListFiles {
                package: args.get_one::<String>("package").unwrap().to_string(),
            }),
            Some(("provides", args)) => OmaCommand::Provides(Provides {
                kw: args.get_one::<String>("pattern").unwrap().to_string(),
            }),
            Some(("fix-broken", args)) => OmaCommand::FixBroken(FixBroken {
                dry_run: args.get_flag("dry_run"),
                dpkg_force_all: args.get_flag("dpkg_force_all"),
            }),
            Some(("pick", args)) => OmaCommand::Pick(PickOptions {
                package: args.get_one::<String>("package").unwrap().to_string(),
                no_fixbroken: args.get_flag("no_fixbroken"),
                no_upgrade: args.get_flag("no_upgrade"),
                dry_run: args.get_flag("dry_run"),
            }),
            Some(("mark", args)) => OmaCommand::Mark(Mark {
                action: match args.get_one::<String>("action").map(|x| x.as_str()) {
                    Some("hold") => MarkAction::Hold(MarkActionArgs {
                        pkgs: pkgs_getter(args).unwrap(),
                    }),
                    Some("unhold") => MarkAction::Unhold(MarkActionArgs {
                        pkgs: pkgs_getter(args).unwrap(),
                    }),
                    Some("auto") => MarkAction::Auto(MarkActionArgs {
                        pkgs: pkgs_getter(args).unwrap(),
                    }),
                    Some("manual") => MarkAction::Manual(MarkActionArgs {
                        pkgs: pkgs_getter(args).unwrap(),
                    }),
                    _ => unreachable!(),
                },
                dry_run: args.get_flag("dry_run"),
            }),
            Some(("command-not-found", args)) => OmaCommand::CommandNotFound(CommandNotFound {
                kw: args.get_one::<String>("package").unwrap().to_string(),
            }),
            Some(("list", args)) => OmaCommand::List(ListOptions {
                packages: pkgs_getter(args),
                all: args.get_flag("all"),
                installed: args.get_flag("installed"),
                upgradable: args.get_flag("upgradable"),
            }),
            Some(("depends", args)) => OmaCommand::Depends(Dep {
                pkgs: pkgs_getter(args).unwrap(),
            }),
            Some(("rdepends", args)) => OmaCommand::Rdepends(Dep {
                pkgs: pkgs_getter(args).unwrap(),
            }),
            Some(("clean", _)) => OmaCommand::Clean,
            Some(("history", _)) => OmaCommand::History,
            _ => unreachable!(),
        };

        Ok(m)
    }
}

pub fn gen_prefix(prefix: &str) -> String {
    if console::measure_text_width(prefix) > (PREFIX_LEN - 1).into() {
        panic!("Line prefix \"{prefix}\" too long!");
    }

    // Make sure the real_prefix has desired PREFIX_LEN in console
    let left_padding_size = (PREFIX_LEN as usize) - 1 - console::measure_text_width(prefix);
    let mut real_prefix: String = " ".repeat(left_padding_size);
    real_prefix.push_str(prefix);
    real_prefix.push(' ');
    real_prefix
}

impl Default for Writer {
    fn default() -> Self {
        Writer {
            term: Term::stderr(),
        }
    }
}

pub struct Writer {
    term: Term,
}

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show_cursor(&self) -> Result<()> {
        self.term.show_cursor()?;
        Ok(())
    }

    pub fn get_max_len(&self) -> u16 {
        self.term.size_checked().unwrap_or((25, 80)).1 - PREFIX_LEN
    }

    pub fn get_height(&self) -> u16 {
        self.term.size_checked().unwrap_or((25, 80)).0
    }

    pub fn get_writer(&self) -> Box<dyn Write> {
        Box::new(self.term.clone())
    }

    fn write_prefix(&self, prefix: &str) -> Result<()> {
        self.term
            .write_str(&gen_prefix(prefix))
            .context("Failed to write prefix to console.")?;
        Ok(())
    }

    pub fn writeln(&self, prefix: &str, msg: &str) -> Result<()> {
        let max_len = self.get_max_len();
        let mut first_run = true;

        let mut msg = msg.to_string();
        // Print msg with left padding
        while !msg.is_empty() {
            let line_msg = console::truncate_str(&msg, max_len.into(), "\n");
            if first_run {
                self.write_prefix(prefix)
                    .context("Failed to write prefix to console.")?;
                first_run = false;
            } else {
                self.write_prefix("")
                    .context("Failed to write prefix to console.")?;
            }
            self.term
                .write_str(&line_msg)
                .context("Failed to write message to console.")?;
            // Remove the already written part, strip ANSI since it can mess everything up
            let mut new_msg = console::strip_ansi_codes(&msg).to_string();
            let line_msg_len = console::measure_text_width(&line_msg);
            new_msg.replace_range(..line_msg_len, "");
            // Swap
            std::mem::swap(&mut msg, &mut new_msg);
        }
        self.term.write_line("")?;

        Ok(())
    }

    // pub fn write_chunks<S: AsRef<str>>(&self, prefix: &str, chunks: &[S]) -> Result<()> {
    //     if chunks.is_empty() {
    //         return Ok(());
    //     }

    //     let max_len: usize = (self.get_max_len() - PREFIX_LEN).into();
    //     // Write prefix first
    //     self.write_prefix(prefix)?;
    //     let mut cur_line_len: usize = PREFIX_LEN.into();
    //     for chunk in chunks {
    //         let chunk = chunk.as_ref();
    //         let chunk_len = console::measure_text_width(chunk);
    //         // If going to overflow the line, create new line
    //         // The `1` is the preceding space
    //         if cur_line_len + chunk_len + 1 > max_len {
    //             self.term.write_str("\n")?;
    //             self.write_prefix("")?;
    //             cur_line_len = 0;
    //         }
    //         self.term.write_str(chunk)?;
    //         self.term.write_str(" ")?;
    //         cur_line_len += chunk_len + 1;
    //     }
    //     // Write a new line
    //     self.term.write_str("\n")?;

    //     Ok(())
    // }
}

// We will ignore write errors in the following macros, since cannot print messages is not an emergency
#[macro_export]
macro_rules! msg {
    ($($arg:tt)+) => {
        if !$crate::DRYRUN.load(Ordering::Relaxed) {
            $crate::WRITER.writeln("", &format!($($arg)+)).ok();
        }
        tracing::info!("{}", &format!($($arg)+));
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => {
        if !$crate::DRYRUN.load(Ordering::Relaxed) {
            $crate::WRITER.writeln(&console::style("DEBUG").dim().to_string(), &format!($($arg)+)).ok();
        }
        tracing::debug!("{}", &format!($($arg)+));
    };
}

#[macro_export]
macro_rules! success {
    ($($arg:tt)+) => {
        if !$crate::DRYRUN.load(Ordering::Relaxed) {
            $crate::WRITER.writeln(&console::style("SUCCESS").green().bold().to_string(), &format!($($arg)+)).ok();
        }
        tracing::info!("{}", &format!($($arg)+));
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => {
        if !$crate::DRYRUN.load(Ordering::Relaxed) {
            $crate::WRITER.writeln(&console::style("INFO").blue().bold().to_string(), &format!($($arg)+)).ok();
        }
        tracing::info!("{}", &format!($($arg)+));
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => {
        if !$crate::DRYRUN.load(Ordering::Relaxed) {
            $crate::WRITER.writeln(&console::style("WARNING").yellow().bold().to_string(), &format!($($arg)+)).ok();
        }
        tracing::warn!("{}", &format!($($arg)+));
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => {
        if !$crate::DRYRUN.load(Ordering::Relaxed) {
            $crate::WRITER.writeln(&console::style("ERROR").red().bold().to_string(), &format!($($arg)+)).ok();
        }
        tracing::error!("{}", &format!($($arg)+));
    };
}

#[macro_export]
macro_rules! due_to {
    ($($arg:tt)+) => {
        if !$crate::DRYRUN.load(Ordering::Relaxed) {
            $crate::WRITER.writeln(&console::style("DUE TO").yellow().bold().to_string(), &format!($($arg)+)).ok();
        }
        tracing::info!("{}", &format!($($arg)+));
    };
}
