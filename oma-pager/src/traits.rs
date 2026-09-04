use ratatui::style::Color;

/// A trait for providing UI text for the pager.
pub trait PagerUIText {
    fn normal_tips(&self, yn_mode: bool) -> String;
    fn search_tips_with_result(&self) -> String;
    fn searct_tips_with_query(&self, query: &str) -> String;
    fn search_tips_with_empty(&self) -> String;
    fn search_tips_not_found(&self) -> String;
}

/// A trait for customizing the pager's colors.
///
/// Implement this trait to control the colors used by the pager, such as the
/// title bar colors. This lets users provide their own theme instead of
/// relying on a terminal theme detection library.
pub trait PagerTheme {
    /// The background color of the title bar.
    fn title_bg_color(&self) -> Color;
    /// The foreground color of the title bar.
    fn title_fg_color(&self) -> Color;
}

/// The exit status of the pager.
pub enum PagerExit {
    NormalExit,
    Sigint,
    DryRun,
}

impl From<PagerExit> for i32 {
    fn from(value: PagerExit) -> Self {
        match value {
            PagerExit::NormalExit => 0,
            PagerExit::Sigint => 130,
            PagerExit::DryRun => 0,
        }
    }
}
