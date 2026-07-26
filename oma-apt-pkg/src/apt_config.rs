use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub(crate) value: String,
    pub(crate) children: HashMap<String, Node>,
}

impl Node {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            children: HashMap::new(),
        }
    }

    pub(crate) fn empty() -> Self {
        Self {
            value: String::new(),
            children: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AptConfig {
    pub(crate) root: Node,
}

impl AptConfig {
    /// Create a fresh, empty APT configuration.
    pub fn new() -> Self {
        Self {
            root: Node::new("/"),
        }
    }

    pub fn init_defaults(&mut self) -> std::io::Result<()> {
        self.set("APT::Architecture", &crate::config_parser::detect_arch()?);
        self.set_list("APT::Build-Essential", "build-essential");
        self.set("APT::Install-Recommends", "true");
        self.set("APT::Install-Suggests", "false");
        self.set(
            "APT::Key::Assert-Pubkey-Algo",
            ">=rsa2048,ed25519,ed448,nistp256,nistp384,nistp512,\
             brainpoolP256r1,brainpoolP320r1,brainpoolP384r1,\
             brainpoolP512r1,secp256k1",
        );
        self.set(
            "APT::Key::Assert-Pubkey-Algo::Next",
            ">=rsa2048,ed25519,ed448,nistp256,nistp384,nistp512",
        );
        self.set(
            "APT::Key::Assert-Pubkey-Algo::Future",
            ">=rsa3072,ed25519,ed448",
        );
        self.set("Dir::State", "var/lib/apt");
        self.set("Dir::State::lists", "lists/");
        self.set("Dir::State::cdroms", "cdroms.list");
        self.set("Dir::State::extended_states", "extended_states");
        self.set("Dir::Cache", "var/cache/apt");
        self.set("Dir::Cache::archives", "archives/");
        self.set("Dir::Cache::srcpkgcache", "srcpkgcache.bin");
        self.set("Dir::Cache::pkgcache", "pkgcache.bin");
        self.set("Dir::Etc", "etc/apt");
        self.set("Dir::Boot", "boot");
        self.set("Dir::Usr", "usr");
        self.set("Dir::Etc::sourcelist", "sources.list");
        self.set("Dir::Etc::sourceparts", "sources.list.d");
        self.set("Dir::Etc::main", "apt.conf");
        self.set("Dir::Etc::netrc", "auth.conf");
        self.set("Dir::Etc::netrcparts", "auth.conf.d");
        self.set("Dir::Etc::parts", "apt.conf.d");
        self.set("Dir::Etc::preferences", "preferences");
        self.set("Dir::Etc::preferencesparts", "preferences.d");
        self.set("Dir::Etc::trustedparts", "trusted.gpg.d");
        self.set("Dir::Bin::methods", "/usr/lib/apt/methods");
        self.set("Dir::Bin::dpkg", "/usr/bin/dpkg");
        self.set("Dir::Media::MountPath", "/media/apt");
        self.set("Dir::Log", "var/log/apt");
        self.set("Dir::Log::Terminal", "term.log");
        self.set("Dir::Log::History", "history.log");
        self.set("Dir::Log::Planner", "eipp.log.xz");
        self.set_list("Dir::Ignore-Files-Silently", "~$");
        self.set_list("Dir::Ignore-Files-Silently", r"\.disabled$");
        self.set_list("Dir::Ignore-Files-Silently", r"\.bak$");
        self.set_list("Dir::Ignore-Files-Silently", r"\.dpkg-[a-z]+$");
        self.set_list("Dir::Ignore-Files-Silently", r"\.ucf-[a-z]+$");
        self.set_list("Dir::Ignore-Files-Silently", r"\.save$");
        self.set_list("Dir::Ignore-Files-Silently", r"\.orig$");
        self.set_list("Dir::Ignore-Files-Silently", r"\.distUpgrade$");
        self.set("Acquire::AllowInsecureRepositories", "false");
        self.set("Acquire::AllowWeakRepositories", "false");
        self.set("Acquire::AllowDowngradeToInsecureRepositories", "false");
        self.set("Acquire::cdrom::mount", "/media/cdrom/");
        self.set("APT::Sandbox::User", "_apt");
        self.set(
            "Acquire::IndexTargets::deb::Packages::MetaKey",
            "$(COMPONENT)/binary-$(ARCHITECTURE)/Packages",
        );
        self.set(
            "Acquire::IndexTargets::deb::Packages::flatMetaKey",
            "Packages",
        );
        self.set(
            "Acquire::IndexTargets::deb::Packages::ShortDescription",
            "Packages",
        );
        self.set(
            "Acquire::IndexTargets::deb::Packages::Description",
            "$(RELEASE)/$(COMPONENT) $(ARCHITECTURE) Packages",
        );
        self.set(
            "Acquire::IndexTargets::deb::Packages::flatDescription",
            "$(RELEASE) Packages",
        );
        self.set("Acquire::IndexTargets::deb::Packages::Optional", "false");
        self.set(
            "Acquire::IndexTargets::deb::Translations::MetaKey",
            "$(COMPONENT)/i18n/Translation-$(LANGUAGE)",
        );
        self.set(
            "Acquire::IndexTargets::deb::Translations::flatMetaKey",
            "$(LANGUAGE)",
        );
        self.set(
            "Acquire::IndexTargets::deb::Translations::ShortDescription",
            "Translation-$(LANGUAGE)",
        );
        self.set(
            "Acquire::IndexTargets::deb::Translations::Description",
            "$(RELEASE)/$(COMPONENT) Translation-$(LANGUAGE)",
        );
        self.set(
            "Acquire::IndexTargets::deb::Translations::flatDescription",
            "$(RELEASE) Translation-$(LANGUAGE)",
        );
        self.set(
            "Acquire::IndexTargets::deb-src::Sources::MetaKey",
            "$(COMPONENT)/source/Sources",
        );
        self.set(
            "Acquire::IndexTargets::deb-src::Sources::flatMetaKey",
            "Sources",
        );
        self.set(
            "Acquire::IndexTargets::deb-src::Sources::ShortDescription",
            "Sources",
        );
        self.set(
            "Acquire::IndexTargets::deb-src::Sources::Description",
            "$(RELEASE)/$(COMPONENT) Sources",
        );
        self.set(
            "Acquire::IndexTargets::deb-src::Sources::flatDescription",
            "$(RELEASE) Sources",
        );
        self.set("Acquire::IndexTargets::deb-src::Sources::Optional", "false");

        Ok(())
    }

    pub fn get(&self, key: &str, default: &str) -> String {
        self.node(key)
            .filter(|n| !n.value.is_empty())
            .map_or_else(|| default.to_string(), |n| n.value.clone())
    }

    pub fn get_file(&self, key: &str, default: &str) -> String {
        match self.node(key) {
            Some(node) if !node.value.is_empty() => {
                let ancestors = self.ancestors(key);
                let mut path = node.value.clone();
                for av in ancestors.iter().rev() {
                    if path.starts_with('/')
                        || path.starts_with("~/")
                        || path.starts_with("./")
                        || path.starts_with("../")
                    {
                        break;
                    }
                    path = format!("{}/{}", av.trim_end_matches('/'), path);
                }
                fl_normalize(&path)
            }
            _ => {
                let dir = self.root.value.as_str();
                if default.starts_with('/') {
                    fl_normalize(default)
                } else {
                    fl_normalize(&format!(
                        "{}/{}",
                        dir.trim_end_matches('/'),
                        default.trim_start_matches('/')
                    ))
                }
            }
        }
    }

    pub fn get_dir(&self, key: &str, default: &str) -> String {
        let mut path = self.get_file(key, default);
        if !path.ends_with('/') {
            path.push('/');
        }
        path
    }

    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        match self.node(key) {
            Some(n) => match n.value.as_str() {
                "1" | "yes" | "true" | "on" => true,
                "0" | "no" | "false" | "off" => false,
                _ => default,
            },
            None => default,
        }
    }

    pub fn exists(&self, key: &str) -> bool {
        self.node(key).is_some_and(|n| !n.value.is_empty())
    }

    pub fn set(&mut self, key: &str, value: &str) {
        let parts: Vec<&str> = key.split("::").collect();
        let mut cur = &mut self.root;
        for part in &parts {
            if *part == "Dir" {
                continue;
            }
            cur = cur
                .children
                .entry(part.to_string())
                .or_insert_with(Node::empty);
        }
        cur.value = value.to_string();
    }

    pub fn set_list(&mut self, key: &str, value: &str) {
        let key = key.strip_suffix("::").unwrap_or(key);
        let parts: Vec<&str> = key.split("::").collect();
        let mut cur = &mut self.root;
        for part in &parts {
            if *part == "Dir" {
                continue;
            }
            cur = cur
                .children
                .entry(part.to_string())
                .or_insert_with(Node::empty);
        }
        cur.children
            .entry(value.to_string())
            .or_insert_with(|| Node::new(value));
    }

    fn node(&self, key: &str) -> Option<&Node> {
        let parts: Vec<&str> = key.split("::").collect();
        let mut cur = &self.root;
        for part in &parts {
            if *part == "Dir" && cur.value == self.root.value {
                continue;
            }
            cur = cur.children.get(*part)?;
        }
        Some(cur)
    }
    /// Return the child key names directly under `key`.
    pub fn keys_under(&self, key: &str) -> Vec<String> {
        self.node(key)
            .map(|n| n.children.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn ancestors(&self, key: &str) -> Vec<&str> {
        let parts: Vec<&str> = key.split("::").collect();
        let mut vals: Vec<&str> = Vec::new();
        let mut cur = &self.root;
        if !cur.value.is_empty() {
            vals.push(cur.value.as_str());
        }
        for part in &parts {
            if *part == "Dir" {
                continue;
            }
            match cur.children.get(*part) {
                Some(c) => {
                    if !c.value.is_empty() {
                        vals.push(c.value.as_str());
                    }
                    cur = c;
                }
                None => break,
            }
        }
        let skip = parts.first().is_some_and(|p| *p == "Dir");
        vals.truncate(if skip { parts.len() - 1 } else { parts.len() });
        vals
    }
}

impl Default for AptConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize a path by removing `//` and `..` and resolving `.` (like APT's
/// `flNormalize`).
fn fl_normalize(path: &str) -> String {
    let mut buf = PathBuf::new();
    for comp in Path::new(path).components() {
        match comp {
            Component::CurDir => continue,
            Component::ParentDir => {
                buf.pop();
            }
            _ => buf.push(comp.as_os_str()),
        }
    }
    buf.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::fl_normalize;
    use crate::AptConfig;

    #[test]
    fn test_fl_normalize() {
        assert_eq!(fl_normalize("/foo//bar"), "/foo/bar");
        assert_eq!(fl_normalize("/foo/./bar"), "/foo/bar");
        assert_eq!(fl_normalize("/foo///bar/./baz"), "/foo/bar/baz");
        assert_eq!(fl_normalize("/foo/bar"), "/foo/bar");
    }

    #[test]
    fn test_load_system_defaults() {
        let mut cfg = AptConfig::new();
        cfg.init_defaults().unwrap();
        assert_eq!(cfg.get("Dir", ""), "/");
        assert_eq!(cfg.get_dir("Dir::State", ""), "/var/lib/apt/");
        assert_eq!(
            cfg.get_file("Dir::Etc::sourcelist", ""),
            "/etc/apt/sources.list"
        );
        assert_eq!(cfg.get_bool("APT::Install-Recommends", false), true);
        assert_eq!(cfg.get_bool("APT::Install-Suggests", true), false);
    }

    #[test]
    fn test_get_file_with_ancestor_resolution() {
        let mut cfg = AptConfig::new();
        cfg.init_defaults().unwrap();
        let path = cfg.get_file("Dir::State::lists", "");
        assert!(
            path == "/var/lib/apt/lists" || path == "/var/lib/apt/lists/",
            "unexpected path: {path}"
        );
        assert_eq!(cfg.get_dir("Dir::State", ""), "/var/lib/apt/");
    }
}
