use std::{env, ffi::OsStr, path::Path, sync::LazyLock};

use clap::{Arg, Args, Command, Parser, Subcommand, builder::Styles, crate_name, crate_version};
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
    fl,
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

static HELP_HEADING: LazyLock<String> = LazyLock::new(|| fl!("clap-command"));
static NEXT_HELP_HEADING: LazyLock<String> = LazyLock::new(|| fl!("clap-argument"));

#[derive(Parser, Debug)]
#[command(
    version,
    about = fl!("clap-about"),
    long_about = None,
    disable_version_flag = true,
    max_term_width = 80,
    after_help = after_help(),
    subcommands = custom_subcmds(),
    subcommand_help_heading = &**HELP_HEADING,
    subcommand_value_name = &**HELP_HEADING,
    next_help_heading = &**NEXT_HELP_HEADING,
    // disable_help_flag = true,
    // disable_help_subcommand = true,
    override_usage = format!(
        "{}oma{} [{}] [{}]",
        Styles::default().get_literal().render(),
        Styles::default().get_literal().render_reset(),
        fl!("clap-argument"),
        fl!("clap-command")
    ),
    help_template = format!("\
{{before-help}}{{about-with-newline}}
{}{}:{} {{usage}}

{{all-args}}{{after-help}}\
    ",
    Styles::default().get_usage().render(),
    fl!("clap-usage"),
    Styles::default().get_usage().render_reset())
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
    #[command(visible_alias = "add", about = fl!("clap-install-help"))]
    Install(Install),
    /// Upgrade packages installed on the system
    #[command(visible_alias = "full-upgrade", about = fl!("clap-upgrade-help"))]
    Upgrade(Upgrade),
    /// Download package(s) from the repository
    #[command(about = fl!("clap-download-help"))]
    Download(Download),
    /// Remove the specified package(s)
    #[command(
        visible_alias = "del",
        visible_alias = "rm",
        visible_alias = "autoremove",
        about = fl!("clap-remove-help"),
    )]
    Remove(Remove),
    /// Refresh repository metadata/catalog
    #[command(about = fl!("clap-refresh-help"))]
    Refresh(Refresh),
    /// Show information on the specified package(s)
    #[command(visible_alias = "info", about = fl!("clap-show-help"))]
    Show(Show),
    /// Search for package(s) available from the repository
    #[command(about = fl!("clap-search-help"))]
    Search(Search),
    /// List files in the specified package
    #[command(about = fl!("clap-files-help"))]
    Files(Files),
    /// Search for package(s) that provide(s) certain patterns in a path
    #[command(about = fl!("clap-provides-help"))]
    Provides(Provides),
    /// Resolve broken dependencies in the system
    #[command(about = fl!("clap-fixbroken-help"))]
    FixBroken(FixBroken),
    /// Install specific version of a package
    #[command(about = fl!("clap-pick-help"))]
    Pick(Pick),
    /// Mark status for one or multiple package(s)
    #[command(about = fl!("clap-mark-help"))]
    Mark(Mark),
    /// List package(s) available from the repository
    List(List),
    /// Lists dependencies of one or multiple packages
    #[command(visible_alias = "dep", about = fl!("clap-depends-help"))]
    Depends(Depends),
    /// List reverse dependency(ies) for the specified package(s)
    #[command(visible_alias = "rdep", about = fl!("clap-rdepends-help"))]
    Rdepends(Rdepends),
    /// Clear downloaded package cache
    #[command(about = fl!("clap-clean-help"))]
    Clean(Clean),
    /// Show a history/log of package changes in the system
    #[command(visible_alias = "log", about = fl!("clap-history-help"))]
    History(History),
    /// Undo system changes operation
    #[command(about = fl!("clap-undo-help"))]
    Undo(Undo),
    /// oma tui interface
    #[command(about = fl!("clap-tui-help"))]
    Tui(Tui),
    /// Print version
    #[command(about = fl!("clap-version-help"))]
    Version(Version),
    #[cfg(feature = "aosc")]
    /// Manage testing topics enrollment
    #[command(visible_alias = "topic", about = fl!("clap-topics-help"))]
    Topics(Topics),
    #[cfg(feature = "mirror")]
    /// Manage Mirrors enrollment
    #[command(visible_alias = "mirrors", about = fl!("clap-mirror-help"))]
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
    #[command(about = fl!("clap-size-analyzer-help"))]
    SizeAnalyzer(SizeAnalyzer),
    /// Display a tree visualization of a dependency graph
    #[command(about = fl!("clap-tree-help"))]
    Tree(Tree),
    /// Why package is installed
    #[command(about = fl!("clap-why-help"))]
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
