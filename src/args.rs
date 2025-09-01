use std::{env, ffi::OsStr, path::Path};

use clap::{Arg, Args, Command, Parser, Subcommand, crate_name, crate_version};
use enum_dispatch::enum_dispatch;
use itertools::Itertools;

use crate::{
    GlobalOptions,
    clean::Clean,
    command_not_found::CommandNotFound,
    config::Config,
    contents_find::{Files, Provides},
    depends::Depends,
    download::Download,
    error::OutputError,
    fix_broken::FixBroken,
    history::{History, Undo},
    install::Install,
    lang::SYSTEM_LANG,
    list::List,
    mark::Mark,
    pick::Pick,
    rdepends::Rdepends,
    refresh::Refresh,
    remove::{Purge, Remove},
    search::Search,
    show::Show,
    subcommand::{
        generate::GenerateManpages,
        size_analyzer::SizeAnalyzer,
        tree::{Tree, Why},
    },
    tui::Tui,
    upgrade::Upgrade,
};

#[cfg(feature = "aosc")]
use crate::topics::Topics;

#[enum_dispatch]
pub(crate) trait CliExecuter {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError>;
}

#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = None,
    disable_version_flag = true,
    max_term_width = 80,
    after_help = after_help(),
    subcommands = custom_subcmds()
)]
pub struct OhManagerAilurus {
    #[command(flatten)]
    pub global: GlobalOptions,
    #[command(subcommand)]
    pub subcmd: Option<SubCmd>,
}

#[enum_dispatch(CliExecuter)]
#[derive(Debug, Subcommand)]
pub enum SubCmd {
    /// Install package(s) from the repository
    #[command(visible_alias = "add")]
    Install(Install),
    /// Upgrade packages installed on the system
    #[command(visible_alias = "full-upgrade")]
    Upgrade(Upgrade),
    /// Download package(s) from the repository
    Download(Download),
    /// Remove the specified package(s)
    #[command(
        visible_alias = "del",
        visible_alias = "rm",
        visible_alias = "autoremove"
    )]
    Remove(Remove),
    /// Refresh repository metadata/catalog
    Refresh(Refresh),
    /// Show information on the specified package(s)
    #[command(visible_alias = "info")]
    Show(Show),
    /// Search for package(s) available from the repository
    Search(Search),
    /// List files in the specified package
    Files(Files),
    /// Search for package(s) that provide(s) certain patterns in a path
    Provides(Provides),
    /// Resolve broken dependencies in the system
    FixBroken(FixBroken),
    /// Install specific version of a package
    Pick(Pick),
    /// Mark status for one or multiple package(s)
    Mark(Mark),
    /// List package(s) available from the repository
    List(List),
    /// Lists dependencies of one or multiple packages
    #[command(visible_alias = "dep")]
    Depends(Depends),
    /// List reverse dependency(ies) for the specified package(s)
    #[command(visible_alias = "rdep")]
    Rdepends(Rdepends),
    /// Clear downloaded package cache
    Clean(Clean),
    /// Show a history/log of package changes in the system
    #[command(visible_alias = "log")]
    History(History),
    /// Undo system changes operation
    Undo(Undo),
    /// Oma tui interface
    Tui(Tui),
    /// Print version
    Version(Version),
    #[cfg(feature = "aosc")]
    /// Manage testing topics enrollment
    #[command(visible_alias = "topic")]
    Topics(Topics),
    #[cfg(feature = "mirror")]
    /// Manage Mirrors enrollment
    #[command(visible_alias = "mirrors")]
    Mirror(crate::mirror::CliMirror),
    /// purge (like apt purge) the specified package(s)
    #[command(hide = true)]
    Purge(Purge),
    /// command-not-found
    #[command(hide = true)]
    CommandNotFound(CommandNotFound),
    #[command(hide = true)]
    /// Generate manpages
    GenerateManpages(GenerateManpages),
    /// Packages size analyzer
    SizeAnalyzer(SizeAnalyzer),
    /// Display a tree visualization of a dependency graph
    Tree(Tree),
    /// Why package is installed
    Why(Why),
}

#[derive(Debug, Args)]
pub struct Version;

impl CliExecuter for Version {
    fn execute(self, _config: &Config, _no_progress: bool) -> Result<i32, OutputError> {
        print_version();
        Ok(0)
    }
}

#[inline]
pub fn print_version() {
    println!("{} {}", crate_name!(), crate_version!());
}

fn after_help() -> &'static str {
    if SYSTEM_LANG.starts_with("zh") {
        "本 oma 具有超级小熊猫力！"
    } else {
        ""
    }
}

fn custom_subcmds() -> Vec<Command> {
    let plugins = list_helpers();
    if let Ok(plugins) = plugins {
        plugins
            .iter()
            .map(|plugin| {
                let name = plugin.strip_prefix("oma-").unwrap_or("???");
                Command::new(name.to_string())
                    .arg(
                        Arg::new("COMMANDS")
                            .required(false)
                            .num_args(1..)
                            .help("Applet specific commands"),
                    )
                    .about("")
            })
            .collect()
    } else {
        vec![]
    }
}

fn list_helpers() -> Result<Vec<String>, anyhow::Error> {
    let mut plugins_dir: Box<dyn Iterator<Item = _>> =
        Box::new(Path::new("/usr/libexec").read_dir()?);

    let plugins_local_dir = Path::new("/usr/local/libexec").read_dir();

    if let Ok(plugins_local_dir) = plugins_local_dir {
        plugins_dir = Box::new(plugins_dir.chain(plugins_local_dir));
    }

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
        .unique()
        .collect();

    Ok(plugins)
}

#[test]
fn test() {
    use clap::CommandFactory;
    use std::ffi::OsString;

    let v: Vec<OsString> = vec!["oma".into(), "-h".into()];
    let app = OhManagerAilurus::parse_from(v);
    OhManagerAilurus::command().print_help().unwrap();

    dbg!(app);
}
