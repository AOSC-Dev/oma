use std::borrow::Cow;

use aho_corasick::AhoCorasick;
use url::Url;

use crate::db::RefreshError;

#[derive(Debug)]
pub(crate) struct DatabaseFilenameReplacer {
    ac: AhoCorasick,
}

impl DatabaseFilenameReplacer {
    const PATTERNS: &'static [&'static str] = &["_", "/", "+", "%3a", "%3A", "@"];
    const REPLACE_WITH: &'static [&'static str] = &["%5f", "_", "%252b", ":", ":", "%40"];

    pub fn new() -> Result<Self, RefreshError> {
        Ok(Self {
            ac: AhoCorasick::new(Self::PATTERNS)?,
        })
    }

    pub fn replace(&self, url: &str) -> Result<String, RefreshError> {
        let url_parsed = Url::parse(url).map_err(|_| RefreshError::InvalidUrl(url.to_string()))?;

        let host = url_parsed.host_str();

        // 不能使用 url_parsed.path()
        // 原因是 "/./" 会被解析器解析为 "/"，而 apt 则不会这样
        let path = if let Some(host) = host {
            url.split_once(host)
                .ok_or_else(|| RefreshError::InvalidUrl(url.to_string()))?
                .1
        } else {
            // file:/// or file:/
            url.strip_prefix("file://")
                .or_else(|| url.strip_prefix("file:"))
                .ok_or_else(|| RefreshError::InvalidUrl(url.to_string()))?
        };

        let url = if let Some(host) = host {
            Cow::Owned(format!("{}{}", host, path))
        } else {
            Cow::Borrowed(path)
        };

        let mut wtr = vec![];

        self.ac
            .try_stream_replace_all(url.as_bytes(), &mut wtr, Self::REPLACE_WITH)
            .map_err(RefreshError::ReplaceAll)?;

        Ok(String::from_utf8_lossy(&wtr).to_string())
    }
}
