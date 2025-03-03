use std::env;

use clap::{Args, Parser, Subcommand, crate_name, crate_version};
use enum_dispatch::enum_dispatch;

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
    generate::Generate,
    history::{History, Undo},
    install::Install,
    list::List,
    mark::Mark,
    pick::Pick,
    pkgnames::Pkgnames,
    rdepends::Rdepends,
    refresh::Refresh,
    remove::{Purge, Remove},
    search::Search,
    show::Show,
    tui::Tui,
    upgrade::Upgrade,
};

#[cfg(feature = "aosc")]
use crate::mirror::CliMirror;

#[cfg(feature = "aosc")]
use crate::topics::Topics;

#[enum_dispatch]
pub(crate) trait CliExecuter {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError>;
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, disable_version_flag = true, max_term_width = 80, after_help = after_help())]
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
    #[cfg(feature = "aosc")]
    /// Manage Mirrors enrollment
    #[command(visible_alias = "mirrors")]
    Mirror(CliMirror),
    /// purge (like apt purge) the specified package(s)
    #[command(hide = true)]
    Purge(Purge),
    /// command-not-found
    #[command(hide = true)]
    CommandNotFound(CommandNotFound),
    #[command(hide = true)]
    /// Pkgnames (used for completion)
    Pkgnames(Pkgnames),
    #[command(hide = true)]
    /// Generate shell completions and manpages
    Generate(Generate),
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
    let Some(lang) = sys_locale::get_locale() else {
        return "";
    };

    if lang.starts_with("zh") {
        "本 oma 具有超级小熊猫力！"
    } else {
        ""
    }
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
