//! This file fix clap_complete::engine::PathCompleter can't complete HOME dir

use std::ffi::OsStr;

use clap_complete::{CompletionCandidate, engine::ValueCompleter};
use clap_lex::OsStrExt;
use dirs::home_dir;
use rustix::path::Arg;
use tracing::debug;

/// Complete a value as a [`std::path::Path`]
///
/// # Example
///
/// ```rust
/// use clap::Parser;
/// use clap_complete::engine::{ArgValueCompleter, PathCompleter};
///
/// #[derive(Debug, Parser)]
/// struct Cli {
///     #[arg(long, add = ArgValueCompleter::new(PathCompleter::file()))]
///     custom: Option<String>,
/// }
/// ```
pub struct PathCompleter {
    current_dir: Option<std::path::PathBuf>,
    #[allow(clippy::type_complexity)]
    filter: Option<Box<dyn Fn(&std::path::Path) -> bool + Send + Sync>>,
    stdio: bool,
}

impl PathCompleter {
    /// Any path is allowed
    pub fn any() -> Self {
        Self {
            filter: None,
            current_dir: None,
            stdio: false,
        }
    }

    /// Complete only files
    pub fn file() -> Self {
        Self::any().filter(|p| p.is_file())
    }

    /// Select which paths should be completed
    pub fn filter(
        mut self,
        filter: impl Fn(&std::path::Path) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.filter = Some(Box::new(filter));
        self
    }
}

impl Default for PathCompleter {
    fn default() -> Self {
        Self::any()
    }
}

impl ValueCompleter for PathCompleter {
    fn complete(&self, current: &OsStr) -> Vec<CompletionCandidate> {
        let filter = self.filter.as_deref().unwrap_or(&|_| true);
        let mut current_dir_actual = None;
        let current_dir = self.current_dir.as_deref().or_else(|| {
            current_dir_actual = std::env::current_dir().ok();
            current_dir_actual.as_deref()
        });

        let mut current = current.to_string_lossy().to_string();

        if current.starts_with("~")
            && let Some(home) = home_dir() {
                current.replace_range(..1, &home.to_string_lossy());
            }

        let mut candidates = complete_path(&current, current_dir, filter);
        if self.stdio && current.is_empty() {
            candidates.push(CompletionCandidate::new("-").help(Some("stdio".into())));
        }
        candidates
    }
}

pub(crate) fn complete_path(
    value_os: &str,
    current_dir: Option<&std::path::Path>,
    is_wanted: &dyn Fn(&std::path::Path) -> bool,
) -> Vec<CompletionCandidate> {
    let mut completions = Vec::new();
    let mut potential = Vec::new();

    let value_path = std::path::Path::new(value_os);
    let (prefix, current) = split_file_name(value_path);
    let current = current.to_string_lossy();
    let search_root = if prefix.is_absolute() {
        prefix.to_owned()
    } else {
        let current_dir = match current_dir {
            Some(current_dir) => current_dir,
            None => {
                // Can't complete without a `current_dir`
                return completions;
            }
        };
        current_dir.join(prefix)
    };
    debug!("complete_path: search_root={search_root:?}, prefix={prefix:?}");

    if value_os.is_empty() && is_wanted(&search_root) {
        completions.push(".".into());
    }

    for entry in std::fs::read_dir(&search_root)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
    {
        let raw_file_name = entry.file_name();
        if !raw_file_name.starts_with(&current) {
            continue;
        }

        if entry.metadata().map(|m| m.is_dir()).unwrap_or(false) {
            let mut suggestion = prefix.join(raw_file_name);
            suggestion.push(""); // Ensure trailing `/`
            let candidate = CompletionCandidate::new(suggestion.as_os_str().to_owned());

            if is_wanted(&entry.path()) {
                completions.push(candidate);
            } else {
                potential.push(candidate);
            }
        } else if is_wanted(&entry.path()) {
            let suggestion = prefix.join(raw_file_name);
            let candidate = CompletionCandidate::new(suggestion.as_os_str().to_owned());
            completions.push(candidate);
        }
    }
    completions.sort();
    potential.sort();
    completions.extend(potential);

    completions
}

fn split_file_name(path: &std::path::Path) -> (&std::path::Path, &OsStr) {
    // Workaround that `Path::new("name/").file_name()` reports `"name"`
    if path_has_name(path) {
        (
            path.parent().unwrap_or_else(|| std::path::Path::new("")),
            path.file_name().expect("not called with `..`"),
        )
    } else {
        (path, Default::default())
    }
}

fn path_has_name(path: &std::path::Path) -> bool {
    let path_bytes = path.as_os_str().as_encoded_bytes();
    let Some(trailing) = path_bytes.last() else {
        return false;
    };
    let trailing = *trailing as char;
    !std::path::is_separator(trailing) && path.file_name().is_some()
}
