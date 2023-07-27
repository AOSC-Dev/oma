use inquire::{
    formatter::MultiOptionFormatter,
    ui::{Color, RenderConfig, StyleSheet, Styled},
    MultiSelect,
};
use oma_console::{console, info};
use oma_topics::Result;
use oma_topics::{list, TopicManager};

fn main() -> Result<()> {
    let mut tm = TopicManager::new()?;
    let mut opt_in = vec![];
    let mut opt_out = vec![];
    let client = reqwest::ClientBuilder::new().user_agent("oma").build()?;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Can not init tokio runtime!");

    let display = list(&mut tm, &client, &rt)?;
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
        "Press [Space]/[Enter] to toggle selection, [q] to apply changes, [Ctrl-c] to abort.",
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
        tm.opt_in(&client, &rt, &i, false, "amd64")?;
    }

    for i in opt_out {
        tm.opt_out(&i, false)?;
    }

    let (tx, rx) = std::sync::mpsc::channel();

    let r = std::thread::spawn(move || -> Result<()> {
        tm.write_enabled(Some(tx), false)?;
        Ok(())
    });

    while let Ok(log) = rx.recv() {
        match log {
            oma_topics::TopicsEvent::Info(s) => {
                info!("{}", s);
            }
        }
    }

    r.join().unwrap().unwrap();

    Ok(())
}
