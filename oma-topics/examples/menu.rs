use inquire::{
    formatter::MultiOptionFormatter,
    ui::{Color, RenderConfig, StyleSheet, Styled},
    MultiSelect,
};

use oma_topics::Result;
use oma_topics::{list, TopicManager};

#[tokio::main]
async fn main() -> Result<()> {
    let mut tm = TopicManager::new().await?;
    let mut opt_in = vec![];
    let mut opt_out = vec![];

    let display = list(&mut tm).await?;
    let all = tm.all.clone();

    let enabled_names = tm.enabled.iter().map(|x| &x.name).collect::<Vec<_>>();
    let all_names = all.iter().map(|x| &x.name).collect::<Vec<_>>();

    let mut default = vec![];

    for (i, c) in all_names.iter().enumerate() {
        if enabled_names.contains(c) {
            default.push(i);
        }
    }

    let formatter: MultiOptionFormatter<&str> = &|a| format!("Activating {} topics", a.len());

    let render_config = RenderConfig {
        selected_checkbox: Styled::new("✔").with_fg(Color::LightGreen),
        help_message: StyleSheet::empty().with_fg(Color::LightBlue),
        unselected_checkbox: Styled::new(" "),
        highlighted_option_prefix: Styled::new(""),
        selected_option: Some(StyleSheet::new().with_fg(Color::DarkCyan)),
        scroll_down_prefix: Styled::new("▼"),
        scroll_up_prefix: Styled::new("▲"),
        ..Default::default()
    };

    let ans = MultiSelect::new(
        "Select topics",
        display.iter().map(|x| x.as_str()).collect(),
    )
    .with_help_message(
        "Press [Space]/[Enter] to toggle selection, [Esc] to apply changes, [Ctrl-c] to abort.",
    )
    .with_formatter(formatter)
    .with_default(&default)
    .with_page_size(20)
    .with_render_config(render_config)
    .prompt()
    .unwrap();

    for i in &ans {
        let index = display.iter().position(|x| x == i).unwrap();
        if !enabled_names.contains(&all_names[index]) {
            opt_in.push(all_names[index].clone());
        }
    }

    for (i, c) in all_names.iter().enumerate() {
        if enabled_names.contains(c) && !ans.contains(&display[i].as_str()) {
            opt_out.push(c.to_string());
        }
    }

    for i in opt_in {
        tm.add(&i, false, "amd64").await?;
    }

    for i in opt_out {
        tm.remove(&i, false)?;
    }

    tm.write_enabled(false).await?;

    Ok(())
}
