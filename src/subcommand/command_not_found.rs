use std::error::Error;
use std::io::Write;

use ahash::AHashMap;
use anyhow::Context;
use clap::Args;
use oma_console::writer::Writer;
use oma_contents::OmaContentsError;
use oma_contents::searcher::{Mode, search};
use oma_logger::{debug, error};
use oma_pm::apt::{OmaApt, OmaAptArgs};
use zbus::{Connection, proxy};

use crate::color::{Action, Colorize};
use crate::config::OmaConfig;
use crate::console::measure_text_width;
use crate::error::OutputError;
use crate::exit_handle::{ExitHandle, ExitStatus};
use crate::utils::get_lists_dir;
use crate::{RT, WRITER, due_to, fl};

use crate::args::CliExecuter;

const FILTER_JARO_NUM: u8 = 204;
/// 部分匹配时最多展示的软件包数量
const MAX_DISPLAY_PKG: usize = 3;
/// 单个软件包内最多展示的相似命令数量，超出部分以省略号略去
const MAX_DISPLAY_CMD: usize = 3;
/// 详情内容相对正文的缩进
const DETAIL_INDENT: &str = "    ";
/// 软件包名与其后正文之间的最小间隔
const LABEL_GAP: usize = 2;

type IndexSet<T> = indexmap::IndexSet<T, ahash::RandomState>;
type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;

#[proxy(
    interface = "io.aosc.Amo1",
    default_service = "io.aosc.Amo",
    default_path = "/io/aosc/Amo"
)]
pub trait Amo {
    async fn get_description(&self, query: &str) -> zbus::Result<String>;
}

#[derive(Debug, Args)]
pub struct CommandNotFound {
    /// Package to query command-not-found
    #[arg(help_heading = &**crate::args::ARG_HELP_HEADING_MUST)]
    keyword: String,
}

impl CliExecuter for CommandNotFound {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let CommandNotFound { keyword } = self;

        print_command_not_found(&keyword, &config)?;

        Ok(ExitHandle::default().status(ExitStatus::Other(127)))
    }
}

/// 检索命令并提供 command-not-found 提示
fn print_command_not_found(keyword: &str, config: &OmaConfig) -> Result<(), OutputError> {
    let mut res = IndexSet::with_hasher(ahash::RandomState::new());

    let cb = |line: (String, String)| {
        if !res.contains(&line) && line.1.starts_with("/usr/bin") {
            res.insert(line);
        }
    };

    let search_res = search(get_lists_dir(), Mode::BinProvides, keyword, cb);

    match search_res {
        Ok(()) if res.is_empty() => {
            print_not_found(&fl!("command-not-found", kw = keyword));
        }
        Ok(()) => {
            let oma_apt_args = OmaAptArgs::builder().build();
            let apt = OmaApt::new(vec![], oma_apt_args, false)?;

            // 按软件包聚合匹配到的命令，保持相似度从高到低的顺序
            let pkgs = group_by_pkg(jaro_nums(res, keyword));

            // 提供该命令的软件包
            let exact = pkgs
                .iter()
                .filter(|(_, cmds)| cmds.iter().any(|(_, score)| *score == u8::MAX))
                .map(|(pkg, _)| pkg.clone())
                .collect::<Vec<_>>();

            let amo = if config.amo && !config.no_check_dbus() {
                RT.handle().block_on(amo_connect()).ok()
            } else {
                None
            };

            let mut desc_cache: AHashMap<String, String> = AHashMap::new();

            print_not_found(&fl!("command-not-found", kw = keyword));
            // 提示行与后续内容之间留一个空行
            blank_line();

            if exact.is_empty() {
                print_section(&fl!("cnf-similar-match"));

                for (pkg, cmds) in pkgs.iter().take(MAX_DISPLAY_PKG) {
                    let desc = get_desc(pkg, amo.as_ref(), &apt, &mut desc_cache)?;

                    print_similar_match(pkg, cmds, desc.as_deref());
                }

                // 相似命令只是被筛过的一部分，安装建议与查看完整匹配合并成一句提示
                // 配色参考 autoremove 的提示：安装建议用 note，查询命令用 secondary
                blank_line();
                if !pkgs.is_empty() {
                    let tip = fl!("cnf-install-tip-similar", query = keyword);
                    let provides_cmd = format!("oma provides --bin {keyword}");

                    write_wrapped_cmd(
                        &tip,
                        &[
                            (install_cmd_span(&tip), Action::Note),
                            (provides_cmd.as_str(), Action::Secondary),
                        ],
                        0,
                    );
                }
            } else {
                print_section(&fl!("cnf-exact-match"));

                // 各软件包共用一列，使描述成列对齐
                let col = detail_col(exact.iter());

                for pkg in &exact {
                    let desc = get_desc(pkg, amo.as_ref(), &apt, &mut desc_cache)?;

                    print_exact_match(pkg, desc.as_deref(), col);
                }

                // 多个软件包都能提供该命令时，提示从列出的结果里挑一个
                let tip = if exact.len() > 1 {
                    fl!("cnf-install-tip-multi", kw = keyword)
                } else {
                    fl!("cnf-install-tip", kw = keyword)
                };

                blank_line();
                print_section(&tip);

                // 多个软件包时给安装命令编号，突出「任选一条」
                for (i, pkg) in exact.iter().enumerate() {
                    let label = (exact.len() > 1).then(|| format!("{}.", i + 1));
                    let col =
                        DETAIL_INDENT.len() + label.as_ref().map_or(0, |label| label.len() + 1);

                    write_wrapped(&format!("oma install {pkg}"), col, label.as_deref(), |s| {
                        s.note_color().bold().to_string()
                    });
                }

                // 结果列表以空行收尾，与 shell 提示行隔开
                blank_line();
            }
        }
        Err(e) => {
            if let OmaContentsError::NoResult = e {
                print_not_found(&fl!("command-not-found", kw = keyword));
            } else {
                let err = OutputError::from(e);
                if !err.to_string().is_empty() {
                    error!("{err}");
                    if let Some(source) = err.source() {
                        due_to!("{source}");
                    }
                }
            }
        }
    }

    Ok(())
}

/// 计算查询命令与软件包中各个命令的相似度（完全一致记为 `u8::MAX`），并按相似度从高到低排序
///
/// 相似度相同时按包名、命令名排序：搜索结果的到达顺序（`rg` 的输出顺序、多线程收集顺序）
/// 并不稳定，不额外定序的话，并列项（例如多个完全匹配的软件包）的先后会在每次运行时漂移。
fn jaro_nums(input: IndexSet<(String, String)>, query: &str) -> Vec<(String, String, u8)> {
    let mut output = vec![];

    for (pkg, file) in input {
        let binary_name = file.split('/').next_back().unwrap_or(&file);

        let num = if pkg == query || binary_name == query {
            u8::MAX
        } else {
            (strsim::jaro_winkler(query, binary_name) * 255.0) as u8
        };

        output.push((pkg, binary_name.to_string(), num));
    }

    output.sort_by(|a, b| {
        b.2.cmp(&a.2)
            .then_with(|| a.0.cmp(&b.0))
            .then_with(|| a.1.cmp(&b.1))
    });

    output
}

/// 按软件包聚合相似命令，软件包及包内命令均保持相似度从高到低的顺序
fn group_by_pkg(entries: Vec<(String, String, u8)>) -> IndexMap<String, Vec<(String, u8)>> {
    let mut pkgs: IndexMap<String, Vec<(String, u8)>> =
        IndexMap::with_hasher(ahash::RandomState::new());

    for (pkg, cmd, score) in entries {
        if score < FILTER_JARO_NUM {
            break;
        }

        let cmds = pkgs.entry(pkg).or_default();

        if !cmds.iter().any(|(name, _)| name == &cmd) {
            cmds.push((cmd, score));
        }
    }

    pkgs
}

/// 查询软件包描述，优先使用 amo 提供的描述，并按需缓存结果
fn get_desc(
    pkg: &str,
    amo: Option<&AmoProxy<'static>>,
    apt: &OmaApt,
    cache: &mut AHashMap<String, String>,
) -> Result<Option<String>, OutputError> {
    if let Some(desc) = cache.get(pkg) {
        return Ok(Some(desc.to_string()));
    }

    let desc = match amo {
        Some(amo) => {
            let desc = RT
                .handle()
                .block_on(amo.get_description(pkg))
                .context("Failed to get description on amo server")?;

            (!desc.is_empty()).then_some(desc)
        }
        None => None,
    };

    let desc = desc
        .or_else(|| {
            apt.cache
                .get(pkg)
                .and_then(|pkg| pkg.candidate())
                .and_then(|candidate| candidate.summary())
        })
        .filter(|desc| !desc.is_empty());

    if let Some(ref desc) = desc {
        cache.insert(pkg.to_string(), desc.to_string());
    }

    Ok(desc)
}

/// 输出顶格的「找不到命令」提示行（红色加粗）
fn print_not_found(text: &str) {
    write_wrapped(text, 0, None, |s| s.error_color().bold().to_string());
}

/// 输出提供该命令的软件包：描述接在软件包名之后，续行与描述起始列对齐
fn print_exact_match(pkg: &str, desc: Option<&str>, col: Option<usize>) {
    let Some(desc) = desc else {
        print_pkg_name(pkg);
        return;
    };

    match col {
        // 描述接在包名之后
        Some(col) => write_wrapped(desc, col, Some(&pkg_label(pkg, col)), |s| {
            s.secondary_color().to_string()
        }),
        // 包名过长时描述另起一行
        None => {
            print_pkg_name(pkg);
            write_wrapped(desc, DETAIL_INDENT.len() * 2, None, |s| {
                s.secondary_color().to_string()
            });
        }
    }
}

/// 输出提供了相似名称命令的软件包：软件包名与命令列表同行，描述缩进一层另起
fn print_similar_match(pkg: &str, cmds: &[(String, u8)], desc: Option<&str>) {
    let cmd_list = fl!("cnf-command-list", cmds = cmds_str(cmds));

    match pkg_detail_col(pkg) {
        // 命令列表接在包名之后
        Some(col) => write_wrapped(&cmd_list, col, Some(&colored_pkg_name(pkg)), |s| {
            s.note_color().to_string()
        }),
        // 包名过长时命令列表另起一行
        None => {
            print_pkg_name(pkg);
            write_wrapped(&cmd_list, DETAIL_INDENT.len() * 2, None, |s| {
                s.note_color().to_string()
            });
        }
    }

    if let Some(desc) = desc {
        write_wrapped(desc, DETAIL_INDENT.len() * 2, None, |s| {
            s.secondary_color().to_string()
        });
    }
}

/// 详情接在软件包名之后时，该软件包自己的正文起始列
///
/// 包名占掉半行时返回 `None`，此时正文另起一行，避免被挤到无法阅读。
fn pkg_detail_col(pkg: &str) -> Option<usize> {
    let col = DETAIL_INDENT.len() + measure_text_width(pkg) + 1;

    (col * 2 <= max_line_len()).then_some(col)
}

/// 一组软件包共用的正文起始列：按最长的包名取列，使各条目的正文成列对齐
///
/// 包名占掉半行时返回 `None`，此时正文另起一行。
fn detail_col<'a>(pkgs: impl Iterator<Item = &'a String>) -> Option<usize> {
    let max_name_width = pkgs.map(|pkg| measure_text_width(pkg)).max()?;
    let col = DETAIL_INDENT.len() + max_name_width + LABEL_GAP;

    (col * 2 <= max_line_len()).then_some(col)
}

/// 标签列中的包名：包名左对齐，补足到共用的正文起始列
///
/// `gen_prefix` 会在标签后补一列空格，这里再多补 `LABEL_GAP - 1` 列，使最长的包名
/// 与正文之间仍留出 `LABEL_GAP` 列间隔。
fn pkg_label(pkg: &str, col: usize) -> String {
    let padding = col - DETAIL_INDENT.len() - 1 - measure_text_width(pkg);

    format!("{}{}", colored_pkg_name(pkg), " ".repeat(padding))
}

/// 拼接软件包内的相似命令，超出 `MAX_DISPLAY_CMD` 的部分以省略号略去
fn cmds_str(cmds: &[(String, u8)]) -> String {
    // 命令列表是 ASCII 命令名，分隔符用半角逗号（不与其它界面的 `comma` 共用）
    let separator = fl!("cnf-command-separator");

    let mut list = cmds
        .iter()
        .take(MAX_DISPLAY_CMD)
        .map(|(cmd, _)| cmd.as_str())
        .collect::<Vec<_>>()
        .join(&separator);

    if cmds.len() > MAX_DISPLAY_CMD {
        list.push_str(&separator);
        list.push_str("...");
    }

    list
}

/// 软件包名：加粗高亮
fn colored_pkg_name(pkg: &str) -> String {
    pkg.emphasis_color().bold().to_string()
}

/// 单独一行输出加粗高亮的软件包名
fn print_pkg_name(pkg: &str) {
    write_wrapped(pkg, DETAIL_INDENT.len(), None, |s| {
        s.emphasis_color().bold().to_string()
    });
}

/// 输出顶格的段落标题，并在其后留一个空行
fn print_section(title: &str) {
    write_wrapped(title, 0, None, |s| s.to_string());

    blank_line();
}

/// 输出空行
fn blank_line() {
    let _ = writeln!(WRITER.get_writer());
}

/// 按 oma Writer 的行宽与缩进行为输出文本
///
/// `col` 是正文起始列：文本自该列起排布，超出「80 列或终端宽度」时自动换行，
/// 续行与正文对齐；`label` 给定时用 Writer 的 `gen_prefix` 补齐首行该列之前的
/// 空白（如包名，显示宽度须小于 `col`），`style` 负责对正文逐行着色。
fn write_wrapped(text: &str, col: usize, label: Option<&str>, style: impl Fn(&str) -> String) {
    let writer = Writer::new(col as u16);
    let term = writer.get_terminal();
    let mut out = writer.get_writer();

    for (i, (prefix, body)) in term.wrap_content("", text).into_iter().enumerate() {
        let lead = match (i, label) {
            (0, Some(label)) => term.gen_prefix(label),
            // `gen_prefix` 在列宽为 0 时会下溢，顶格输出时不做填充
            _ if col == 0 => String::new(),
            _ => term.gen_prefix(prefix),
        };

        let _ = writeln!(out, "{lead}{}", style(body.trim_end()));
    }
}

/// 输出文本，并把其中给定的各个命令按各自的配色标出
///
/// `cmds` 给出命令文本与配色（目前用到 `Note` 与 `Secondary`）；折行仍然交给
/// Writer，再逐行把内容对回原文：折行只会在边界处丢弃空白字符，对位成功后即可
/// 知道每行里哪些片段是命令，按行着色（命令跨行时两行各自着色）。
fn write_wrapped_cmd(text: &str, cmds: &[(&str, Action)], col: usize) {
    // 各命令在原文中的范围，按位置排序
    let mut spans = cmds
        .iter()
        .filter_map(|(cmd, action)| {
            text.find(*cmd)
                .map(|start| (start, start + cmd.len(), action))
        })
        .collect::<Vec<_>>();

    if spans.is_empty() {
        // 消息里找不到命令，按普通文本输出
        write_wrapped(text, col, None, |s| s.to_string());
        return;
    }

    spans.sort_unstable_by_key(|(start, _, _)| *start);

    let writer = Writer::new(col as u16);
    let term = writer.get_terminal();
    let mut out = writer.get_writer();
    let lead = match col {
        0 => String::new(),
        _ => term.gen_prefix(""),
    };

    // 原文中已经输出到的位置，用于把每一行对回原文
    let mut cursor = 0;

    for (_, body) in term.wrap_content("", text) {
        let line = body.trim_end();
        let rest = &text[cursor..];
        let start = cursor + (rest.len() - rest.trim_start().len());
        let end = start + line.len();

        // 行内容能对回原文时才有着色把握；对不上就按普通文本输出
        if text.get(start..end) != Some(line) {
            let _ = writeln!(out, "{lead}{line}");
            continue;
        }

        cursor = end;

        let _ = write!(out, "{lead}");

        // 本行内逐段输出：命令段着色，其余按普通文本
        let mut pos = start;

        for (span_start, span_end, action) in &spans {
            let lo = (*span_start).max(start).max(pos);
            let hi = (*span_end).min(end);

            if lo >= hi {
                continue;
            }

            let command = &text[lo..hi];
            let styled = match action {
                Action::Note => command.note_color().to_string(),
                Action::Secondary => command.secondary_color().to_string(),
                _ => command.to_string(),
            };

            let _ = write!(out, "{}{}", &text[pos..lo], styled);
            pos = hi;
        }

        let _ = writeln!(out, "{}", &text[pos..end]);
    }
}

/// 从安装提示里取出待着色的命令形式，即 `oma install <软件包名称>`（含占位符）
///
/// 占位符文本随翻译变化，这里取「oma install」到其后第一个 `>` 的范围；
/// 找不到时退化为只着色 `oma install`。
fn install_cmd_span(text: &str) -> &str {
    let Some(start) = text.find("oma install") else {
        return "oma install";
    };

    let command = &text[start..];

    match command.find('>') {
        Some(end) => &command[..=end],
        None => "oma install",
    }
}

/// 界面最大行宽：不超过 80 列，也不超过终端宽度
fn max_line_len() -> usize {
    WRITER.get_max_len().into()
}

async fn amo_connect() -> anyhow::Result<AmoProxy<'static>> {
    let conn = Connection::system().await?;

    let peer_proxy = zbus::fdo::PeerProxy::builder(&conn)
        .destination("io.aosc.Amo")?
        .path("/io/aosc/Amo")?
        .build()
        .await?;

    peer_proxy
        .ping()
        .await
        .inspect_err(|e| debug!("Failed to connect amo: {e}"))?;

    let amo = AmoProxy::new(&conn).await?;

    Ok(amo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmds_str() {
        let cmds = |cmds: &[&str]| {
            cmds.iter()
                .map(|cmd| (cmd.to_string(), u8::MAX))
                .collect::<Vec<_>>()
        };

        assert_eq!(cmds_str(&cmds(&["ffplay"])), "ffplay");
        assert_eq!(
            cmds_str(&cmds(&["ffplay", "ffplay2", "ffplay3"])),
            "ffplay, ffplay2, ffplay3"
        );
        assert_eq!(
            cmds_str(&cmds(&["ffplay", "ffplay2", "ffplay3", "ffplay4"])),
            "ffplay, ffplay2, ffplay3, ..."
        );
    }

    #[test]
    fn test_jaro_nums_tie_break_by_name() {
        // 并列（完全匹配）时按包名排序，避免搜索结果的到达顺序影响输出
        let mut set = IndexSet::with_hasher(ahash::RandomState::new());
        set.insert((
            "yakuake-trinity".to_string(),
            "/usr/bin/yakuake".to_string(),
        ));
        set.insert(("yakuake".to_string(), "/usr/bin/yakuake".to_string()));

        let res = jaro_nums(set, "yakuake");

        assert_eq!(res[0].0, "yakuake");
        assert_eq!(res[1].0, "yakuake-trinity");
    }

    #[test]
    fn test_group_by_pkg() {
        let entries = vec![
            ("ffmpeg".to_string(), "ffplay".to_string(), u8::MAX),
            ("ffmpeg".to_string(), "ffplay2".to_string(), 220),
            ("gstreamer".to_string(), "fftwplay".to_string(), 210),
            // 同一软件包内的同一命令只保留一次
            ("ffmpeg".to_string(), "ffplay".to_string(), 220),
            // 相似度过低的软件包将被忽略
            (
                "too-low".to_string(),
                "unrelated".to_string(),
                FILTER_JARO_NUM - 1,
            ),
        ];

        let pkgs = group_by_pkg(entries);

        assert_eq!(pkgs.len(), 2);
        assert_eq!(
            pkgs.get("ffmpeg").unwrap(),
            &vec![
                ("ffplay".to_string(), u8::MAX),
                ("ffplay2".to_string(), 220)
            ]
        );
        assert_eq!(
            pkgs.get("gstreamer").unwrap(),
            &vec![("fftwplay".to_string(), 210)]
        );
        // 软件包按首次出现（即最高相似度）的顺序排列
        assert_eq!(pkgs.keys().collect::<Vec<_>>(), vec!["ffmpeg", "gstreamer"]);
    }
}
