use clap::{Parser, ArgAction, Subcommand};

#[derive(Parser, Debug)]
#[clap(about, version, author)]
pub struct Args {
    #[clap(subcommand)]
    pub subcommand: OmaCommand,
    #[arg(long, hide = true, action = ArgAction::Count)]
    pub ailurus: u8,
    /// Dry-run oma
    #[arg(long, short = 'd')]
    pub dry_run: bool,
    /// Debug mode (for --dry-run argument)
    #[arg(long)]
    pub debug: bool,
}

#[derive(Subcommand, Debug)]
pub enum OmaCommand {
    /// Install Package
    Install(InstallOptions),
    /// Update Package
    #[clap(alias = "full-upgrade", alias = "dist-upgrade")]
    Upgrade(UpgradeOptions),
    /// Download Package
    Download(Download),
    /// Delete Package
    #[clap(alias = "delete", alias = "purge")]
    Remove(RemoveOptions),
    /// Refresh Package database
    #[clap(alias = "update")]
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
    #[clap(hide = true)]
    CommandNotFound(CommandNotFound),
    /// List of packages
    List(ListOptions),
    /// Check package dependencies
    #[clap(alias = "dep")]
    Depends(Dep),
    /// Check package reverse dependencies
    #[clap(alias = "rdep")]
    Rdepends(Dep),
    /// Clean downloaded packages
    Clean,
    /// See omakase log
    Log,
}

#[derive(Parser, Debug)]
pub struct Dep {
    /// Package(s) name
    pub pkgs: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct CommandNotFound {
    pub kw: String,
}

#[derive(Parser, Debug)]
pub struct FixBroken {
    /// Dry-run oma
    #[arg(long, short = 'd')]
    pub dry_run: bool,
}

#[derive(Parser, Debug)]
pub struct Download {
    /// Package(s) name
    pub packages: Vec<String>,
    /// Download to path
    #[arg(long, short)]
    pub path: Option<String>,
}

#[derive(Parser, Debug, Clone)]
pub struct InstallOptions {
    /// Package(s) name
    pub packages: Vec<String>,
    /// Install package(s) debug symbol
    #[arg(long, alias = "dbg")]
    pub install_dbg: bool,
    /// Reinstall package(s)
    #[arg(long)]
    pub reinstall: bool,
    /// Do not try fix package depends broken status
    #[arg(long)]
    pub no_fixbroken: bool,
    /// Do not refresh packages database
    #[arg(long)]
    pub no_upgrade: bool,
    /// Automatic run oma install
    #[arg(long, short = 'y')]
    pub yes: bool,
    /// Force install packages for can't resolve depends
    #[arg(long)]
    pub force_yes: bool,
    /// Install package use dpkg --force-confnew
    #[arg(long)]
    pub force_confnew: bool,
    /// Dry-run oma
    #[arg(long, short = 'd')]
    pub dry_run: bool,
}

#[derive(Parser, Debug, Clone)]
pub struct UpgradeOptions {
    /// Package(s) name
    pub packages: Vec<String>,
    /// Automatic run oma install
    #[arg(long, short = 'y')]
    pub yes: bool,
    /// Force install packages for can't resolve depends
    #[arg(long)]
    pub force_yes: bool,
    /// Install package use dpkg --force-confnew
    #[arg(long)]
    pub force_confnew: bool,
    /// Dry-run oma
    #[arg(long, short = 'd')]
    pub dry_run: bool,
}

#[derive(Parser, Debug)]
pub struct ListFiles {
    /// Package name
    pub package: String,
}

#[derive(Parser, Debug, Clone)]
pub struct PickOptions {
    /// Package name
    pub package: String,
    /// Do not try fix package depends broken status
    #[arg(long)]
    pub no_fixbroken: bool,
    /// Do not refresh packages database
    #[arg(long)]
    pub no_upgrade: bool,
    /// Dry-run oma
    #[arg(long, short = 'd')]
    pub dry_run: bool,
}

#[derive(Parser, Debug)]
pub struct Provides {
    /// Search keyword
    pub kw: String,
}

#[derive(Parser, Debug, Clone)]
pub struct RemoveOptions {
    /// Package(s) name
    pub packages: Vec<String>,
    /// Automatic run oma install
    #[arg(long, short = 'y')]
    pub yes: bool,
    /// Force install packages for can't resolve depends
    #[arg(long)]
    pub force_yes: bool,
    /// Keep package config
    #[arg(long)]
    pub keep_config: bool,
    /// Dry-run oma
    #[arg(long, short = 'd')]
    pub dry_run: bool,
}

#[derive(Parser, Debug)]
pub struct Show {
    /// Package(s) name
    pub packages: Vec<String>,
    #[arg(long, short = 'a')]
    pub is_all: bool,
}

#[derive(Parser, Debug)]
pub struct Search {
    /// Search keyword(s)
    pub keyword: Vec<String>,
}

#[derive(Parser, Debug, Clone)]
pub struct Mark {
    #[clap(subcommand)]
    pub action: MarkAction,
    /// Dry-run oma
    #[arg(long, short = 'd')]
    pub dry_run: bool,
}


#[derive(Subcommand, Debug, Clone)]
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

#[derive(Parser, Debug, Clone)]
pub struct MarkActionArgs {
    pub pkgs: Vec<String>,
}


#[derive(Parser, Debug)]
pub struct ListOptions {
    pub packages: Option<Vec<String>>,
    #[arg(long, short = 'a')]
    pub all: bool,
    #[arg(long, short = 'i')]
    pub installed: bool,
    #[arg(long, short = 'u')]
    pub upgradable: bool,
}
