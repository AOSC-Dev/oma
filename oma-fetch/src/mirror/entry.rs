use std::collections::HashMap;

use oma_logger::debug;

use super::target::{MirrorTarget, MirrorTransport};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MirrorEntry {
    pub(super) uri: String,
    pub(super) priority: u64,
    /// Tag names are normalized like apt's `MirrorInfo`: `arch` becomes
    /// `Architecture`, `suite`/`codename` are also recorded under `Release`,
    /// and any other name is capitalized.
    pub(super) tags: HashMap<String, Vec<String>>,
}

impl MirrorEntry {
    pub(super) fn matches(&self, target: &MirrorTarget<'_>) -> bool {
        tag_matches(self, "Architecture", target.arch)
            && tag_matches(self, "Component", target.component)
            && tag_matches(self, "Release", target.release)
            && tag_matches(self, "Suite", target.suite)
            && tag_matches(self, "Codename", target.codename)
            && tag_matches(self, "Language", target.language)
            && tag_matches(self, "Type", target.kind.apt_type())
    }
}

fn tag_matches(entry: &MirrorEntry, key: &str, value: Option<&str>) -> bool {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return true;
    };

    match entry.tags.get(key) {
        // A mirror without the tag is usable for every request.
        None => true,
        Some(values) => values.iter().any(|tag| tag == value),
    }
}

pub(super) fn parse_entries(text: &str) -> Vec<MirrorEntry> {
    let mut entries = Vec::new();

    for line in text.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (uri, tags) = match line.split_once('\t') {
            Some((uri, tags)) => (uri.trim(), tags),
            None => (line, ""),
        };

        if uri.is_empty() {
            continue;
        }

        let mut entry = MirrorEntry {
            uri: uri.to_string(),
            priority: u64::MAX,
            tags: HashMap::new(),
        };

        for tag in tags.split_whitespace() {
            let Some((name, value)) = tag.split_once(':') else {
                continue;
            };

            if name.is_empty() {
                continue;
            }

            match name {
                "priority" => {
                    entry.priority = value.parse().unwrap_or_else(|_| {
                        debug!("Ignoring invalid mirror priority {value:?} for {uri}");
                        u64::MAX
                    });
                }
                "arch" => push_tag(&mut entry.tags, "Architecture", value),
                "lang" => push_tag(&mut entry.tags, "Language", value),
                "suite" => {
                    push_tag(&mut entry.tags, "Suite", value);
                    // apt records suite/codename under Release as well, so a
                    // mirror tagged either way can serve index requests
                    // (which only carry Target-Release).
                    push_tag(&mut entry.tags, "Release", value);
                }
                "codename" => {
                    push_tag(&mut entry.tags, "Codename", value);
                    push_tag(&mut entry.tags, "Release", value);
                }
                _ => {
                    let mut key = name.to_string();
                    if let Some(first) = key.get_mut(0..1) {
                        first.make_ascii_uppercase();
                    }
                    push_tag_owned(&mut entry.tags, key, value);
                }
            }
        }

        entries.push(entry);
    }

    entries
}

fn push_tag(tags: &mut HashMap<String, Vec<String>>, key: &str, value: &str) {
    if let Some(values) = tags.get_mut(key) {
        values.push(value.to_string());
    } else {
        tags.insert(key.to_string(), vec![value.to_string()]);
    }
}

fn push_tag_owned(tags: &mut HashMap<String, Vec<String>>, key: String, value: &str) {
    if let Some(values) = tags.get_mut(key.as_str()) {
        values.push(value.to_string());
    } else {
        tags.insert(key, vec![value.to_string()]);
    }
}

pub(super) fn entry_supported(entry: &MirrorEntry, from_network: bool) -> bool {
    let Some((scheme, _)) = entry.uri.split_once(':') else {
        return false;
    };

    if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
        true
    } else if scheme.eq_ignore_ascii_case("file") {
        // A network-fetched list must not point at local files
        // (CVE-2018-0501).
        !from_network
    } else {
        false
    }
}

pub(super) fn transport_of(uri: &str) -> Option<MirrorTransport> {
    let (scheme, _) = uri.split_once(':')?;

    if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
        Some(MirrorTransport::Http)
    } else if scheme.eq_ignore_ascii_case("file") {
        Some(MirrorTransport::File)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mirror::MirrorTransport;

    #[test]
    fn parses_entry_tags_like_apt() {
        let entries = parse_entries(
            "# comment\n\
             http://m1.example.com/debian/\tpriority:1\tarch:amd64\tsuite:bookworm\ttype:index\n\
             http://m2.example.com/debian/\tcodename:trixie\tcomponent:main\tlang:zh_CN\n\
             file:/srv/repo\n",
        );

        assert_eq!(entries.len(), 3);

        let m1 = &entries[0];
        assert_eq!(m1.priority, 1);
        assert_eq!(m1.tags["Architecture"], vec!["amd64".to_string()]);
        assert_eq!(m1.tags["Suite"], vec!["bookworm".to_string()]);
        assert_eq!(m1.tags["Release"], vec!["bookworm".to_string()]);
        assert_eq!(m1.tags["Type"], vec!["index".to_string()]);

        let m2 = &entries[1];
        assert_eq!(m2.priority, u64::MAX);
        assert_eq!(m2.tags["Codename"], vec!["trixie".to_string()]);
        assert_eq!(m2.tags["Release"], vec!["trixie".to_string()]);
        assert_eq!(m2.tags["Component"], vec!["main".to_string()]);
        assert_eq!(m2.tags["Language"], vec!["zh_CN".to_string()]);

        assert_eq!(transport_of(&entries[2].uri), Some(MirrorTransport::File));
    }

    #[test]
    fn filters_unsupported_transports() {
        let entries = parse_entries(
            "ftp://m1.example.com/\n\
             copy:/m2/\n\
             mirror+http://m3.example.com/list\n\
             http://ok.example.com/\n\
             file:/srv/repo\n",
        );

        assert_eq!(entries.len(), 5);

        // A local list may contain local mirrors; a remote one may not.
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry_supported(entry, false))
                .count(),
            2
        );

        let remote: Vec<_> = entries
            .iter()
            .filter(|entry| entry_supported(entry, true))
            .collect();
        assert_eq!(remote.len(), 1);
        assert_eq!(remote[0].uri, "http://ok.example.com/");
    }
}
