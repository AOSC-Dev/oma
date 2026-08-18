use std::env;
use std::io::{IsTerminal, stderr, stdin};
use std::sync::OnceLock;
use std::sync::atomic::Ordering;
use std::time::Duration;

use oma_console::print::termbg::Theme;
use oma_console::{
    console::{self, Color, StyledObject, style},
    print::termbg,
};
use rustix::stdio::stdout;
use spdlog::debug;

use crate::NO_COLOR;
use crate::dbus::is_ssh_from_loginctl;

/// Time budget for probing the terminal theme (see `should_follow_terminal`).
const TERMBG_TIMEOUT: Duration = Duration::from_millis(100);

/// A color role used to theme output for a given semantic.
#[allow(dead_code)] // PendingBg is reserved for future background usage (e.g. pending bar).
pub enum Action {
    Emphasis,
    Foreground,
    Secondary,
    EmphasisSecondary,
    Warn,
    Purple,
    Note,
    UpgradeTips,
    PendingBg,
}

impl Action {
    /// The 256-color palette entry for this role under `theme`.
    fn palette(&self, theme: Theme) -> u8 {
        match (self, theme) {
            (Action::Emphasis, Theme::Dark) => 148,
            (Action::Emphasis, Theme::Light) => 142,
            (Action::Foreground, _) => 72,
            (Action::Secondary, Theme::Dark) => 182,
            (Action::Secondary, Theme::Light) => 167,
            (Action::EmphasisSecondary, Theme::Dark) => 114,
            (Action::EmphasisSecondary, Theme::Light) => 106,
            (Action::Warn, Theme::Dark) => 214,
            (Action::Warn, Theme::Light) => 208,
            (Action::Purple, _) => 141,
            (Action::Note, Theme::Dark) => 178,
            (Action::Note, Theme::Light) => 172,
            (Action::UpgradeTips, Theme::Dark) => 87,
            (Action::UpgradeTips, Theme::Light) => 63,
            (Action::PendingBg, Theme::Dark) => 25,
            (Action::PendingBg, Theme::Light) => 189,
        }
    }
}

/// Resolved terminal theme, detected once at startup. `None` means fall back
/// to the terminal's default (named) colors.
static TERM_THEME: OnceLock<Option<Theme>> = OnceLock::new();

/// Initialize oma's color system at startup: disable colors if requested,
/// then detect the terminal theme and decide whether oma's own palette is
/// used.
pub fn init_color(no_color: bool, follow_terminal_color: bool) {
    if no_color {
        unsafe { env::set_var("NO_COLOR", "1") };
        console::set_colors_enabled(false);
        NO_COLOR.store(true, Ordering::Relaxed);
    }

    let theme = if should_follow_terminal(no_color, follow_terminal_color) {
        None
    } else {
        termbg::theme(TERMBG_TIMEOUT)
            .map_err(|e| {
                debug!(
                    "Failed to apply oma color schemes, falling back to default terminal colors: {e:?}."
                );
                e
            })
            .ok()
    };

    let _ = TERM_THEME.set(theme);
}

/// Whether oma should fall back to the terminal's default colors instead of
/// using its own palette.
///
/// FIXME: Marking latency limits for oma's terminal color queries (via
/// termbg). On slower terminals - i.e., SSH and unaccelerated
/// graphical environments, any colored interfaces in oma may return a
/// terminal color query string in the returned shell, confusing users.
///
///   (ssh)root@LoongUnion1 [ `~` ] ? 11;rgb:2323/2626/2727
///
/// Following advice from termbg here. Add latency limits to avoid this
/// strange output on slower terminals.
///
/// For further investigation, we have some remaining questions:
///
/// 1. Why 100ms? We see that the termbg-based procs project using the
///    same latency limit to workaround the aforementioned issue.
///    It should be noted that this is nothing more than a "magic
///    number" that we have tested to work.
/// 2. The true cause or reproducing conditions for this issue is not
///    yet clear, we found the same issue on a slower machine (Loongson
///    3B4000) in a nearby datacenter (`~50ms`) with a faster one
///    (Loongson 3C5000), which does not exhibit the issue; as well as
///    on a faster machine (AMD EPYC 7H12) with high latency (`~450ms`).
///
/// Ref: https://github.com/dalance/procs/issues/221
/// Ref: https://github.com/dalance/procs/commit/83305be6fb431695a070524328b66c7107ce98f3
fn should_follow_terminal(no_color: bool, follow_terminal_color: bool) -> bool {
    let mut follow = follow_terminal_color;

    if !stdout().is_terminal() || !stderr().is_terminal() || !stdin().is_terminal() || no_color {
        follow = true;
    } else if env::var("SSH_CONNECTION").is_ok() || is_ssh_from_loginctl() {
        debug!(
            "You are running oma in an SSH session, using default terminal colors to avoid latency."
        );
        follow = true;
    } else if env::var("TERM").is_err() || termbg::terminal() != termbg::Terminal::XtermCompatible {
        debug!("Your terminal is: {:?}", termbg::terminal());
        debug!(
            "Unknown or unsupported terminal ($TERM is empty or unsupported) detected, using default terminal colors to avoid latency."
        );
        follow = true;
    } else if let Ok(latency) = termbg::latency(Duration::from_millis(1000)) {
        debug!("latency: {:?}", latency);
        if latency * 2 > TERMBG_TIMEOUT {
            debug!(
                "Terminal latency is too long, falling back to default terminal colors, latency: {:?}.",
                latency
            );
            follow = true;
        }
    } else {
        debug!("Terminal latency is too long, falling back to default terminal colors.");
        follow = true;
    }

    follow
}

/// The resolved terminal theme (dark/light), if a theme was detected.
pub fn color_theme() -> Option<Theme> {
    *TERM_THEME.get().unwrap_or(&None)
}

/// Style `input` with the color palette of the resolved terminal theme.
pub(crate) fn color_str<D>(input: D, color: Action) -> StyledObject<D> {
    match color_theme() {
        Some(theme) => match color {
            x @ Action::PendingBg => style(input).bg(Color::Color256(x.palette(theme))).bold(),
            x => style(input).color256(x.palette(theme)),
        },
        None => term_color(input, color),
    }
}

/// Fallback styling using the terminal's default (named) colors.
fn term_color<D>(input: D, color: Action) -> StyledObject<D> {
    match color {
        Action::Emphasis => style(input).green(),
        Action::Secondary => style(input).dim(),
        Action::EmphasisSecondary => style(input).cyan(),
        Action::Warn => style(input).yellow().bold(),
        Action::Purple => style(input).magenta(),
        Action::Note => style(input).yellow(),
        Action::Foreground => style(input).cyan().bold(),
        Action::UpgradeTips => style(input).blue().bold(),
        Action::PendingBg => style(input).bg(Color::Blue).bold(),
    }
}

/// Extension trait providing semantic color methods on any value, e.g.
/// `name.warn_color()`, backed by [`color_str`].
pub trait Colorize {
    fn emphasis_color(self) -> StyledObject<Self>
    where
        Self: Sized;
    fn foreground_color(self) -> StyledObject<Self>
    where
        Self: Sized;
    fn secondary_color(self) -> StyledObject<Self>
    where
        Self: Sized;
    fn emphasis_secondary_color(self) -> StyledObject<Self>
    where
        Self: Sized;
    fn warn_color(self) -> StyledObject<Self>
    where
        Self: Sized;
    fn purple_color(self) -> StyledObject<Self>
    where
        Self: Sized;
    fn note_color(self) -> StyledObject<Self>
    where
        Self: Sized;
    fn upgrade_tips_color(self) -> StyledObject<Self>
    where
        Self: Sized;
    #[allow(dead_code)] // reserved for future background usage (e.g. pending bar)
    fn pending_bg_color(self) -> StyledObject<Self>
    where
        Self: Sized;
}

impl<T> Colorize for T {
    fn emphasis_color(self) -> StyledObject<Self> {
        color_str(self, Action::Emphasis)
    }
    fn foreground_color(self) -> StyledObject<Self> {
        color_str(self, Action::Foreground)
    }
    fn secondary_color(self) -> StyledObject<Self> {
        color_str(self, Action::Secondary)
    }
    fn emphasis_secondary_color(self) -> StyledObject<Self> {
        color_str(self, Action::EmphasisSecondary)
    }
    fn warn_color(self) -> StyledObject<Self> {
        color_str(self, Action::Warn)
    }
    fn purple_color(self) -> StyledObject<Self> {
        color_str(self, Action::Purple)
    }
    fn note_color(self) -> StyledObject<Self> {
        color_str(self, Action::Note)
    }
    fn upgrade_tips_color(self) -> StyledObject<Self> {
        color_str(self, Action::UpgradeTips)
    }
    fn pending_bg_color(self) -> StyledObject<Self> {
        color_str(self, Action::PendingBg)
    }
}
