use indexmap::IndexMap;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub(crate) value: String,
    pub(crate) children: IndexMap<String, Node>,
}

impl Node {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            children: IndexMap::new(),
        }
    }

    pub(crate) fn empty() -> Self {
        Self {
            value: String::new(),
            children: IndexMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AptConfig {
    pub(crate) root: Node,
}

impl AptConfig {
    /// Create a empty APT configuration tree.
    pub fn new() -> Self {
        Self {
            root: Node::new("/"),
        }
    }

    /// Initialize the default APT configuration tree.
    ///
    /// The values mirror apt's own built-in defaults, so paths and
    /// behaviours match the reference implementation. Line numbers refer
    /// to the apt source at `3.2.0` (Debian/apt):
    ///
    /// - `apt-pkg/init.cc` — `pkgInitConfig()` (starts at L113):
    ///   - L116 `APT::Architecture` (apt sets the compile-time `COMMON_ARCH`;
    ///     this crate instead detects it via `config_parser::detect_arch()`,
    ///     i.e. the output of `dpkg --print-architecture`)
    ///   - L117-118 `APT::Build-Essential`
    ///   - L119-120 `APT::Install-Recommends` / `APT::Install-Suggests`
    ///   - L121-123 `APT::Key::Assert-Pubkey-Algo` (+ `::Next` / `::Future`,
    ///     apt 2.7+ key-algorithm enforcement)
    ///   - L127-129 `Dir::State` / `::lists` / `::cdroms`
    ///   - L132-135 `Dir::Cache` / `::archives` / `::srcpkgcache` / `::pkgcache`
    ///   - L138-149 `Dir::Etc` tree, L150 `Dir::Bin::methods`,
    ///     L153 `Dir::Media::MountPath`
    ///   - L156-159 `Dir::Log` / `::Terminal` / `::History` / `::Planner`
    ///   - L161-168 `Dir::Ignore-Files-Silently` patterns
    ///   - L171-173 `Acquire::AllowInsecureRepositories` /
    ///     `AllowWeakRepositories` / `AllowDowngradeToInsecureRepositories`
    ///   - L176 `Acquire::cdrom::mount`, L179 `APT::Sandbox::User`
    ///   - L181-197 `Acquire::IndexTargets`: `deb::Packages` (L181-186),
    ///     `deb::Translations` (L187-191), `deb-src::Sources` (L192-197)
    /// - `apt-pkg/deb/debsystem.cc` — `debSystem::Initialize()` (starts at
    ///   L288): L293 `Dir::State::extended_states`, L296 `Dir::Bin::dpkg`
    ///
    /// Every key here can be overridden by the system configuration
    /// (`load_system`, e.g. `/etc/apt/apt.conf.d` and friends).
    pub fn init_defaults(&mut self) -> std::io::Result<()> {
        self.set("APT::Architecture", &crate::config_parser::detect_arch()?);
        self.set_list("APT::Build-Essential", "build-essential");
        self.set("APT::Install-Recommends", "true");
        self.set("APT::Install-Suggests", "false");
        // apt 2.7+ key-algorithm enforcement (apt-pkg/init.cc:121-123).
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
        // apt's built-in index targets (deb Packages / Translations,
        // deb-src Sources), from apt-pkg/init.cc:181-197 (pkgInitConfig).
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

    /// Get value from key, or fallback to default
    pub fn get(&self, key: &str, default: &str) -> String {
        self.node(key)
            .filter(|n| !n.value.is_empty())
            .map_or_else(|| default.to_string(), |n| n.value.clone())
    }

    /// Get file path from key, or fallback to default.
    ///
    /// Resolves ancestor values (using `PathBuf::push` which naturally
    /// handles absolute-path replacement), prepends `RootDir` (if set),
    /// and normalizes via [`fl_normalize`].
    pub fn get_file(&self, key: &str, default: &str) -> String {
        let mut buf = PathBuf::new();

        match self.node(key) {
            Some(node) if !node.value.is_empty() => {
                for av in self.ancestors(key) {
                    buf.push(av);
                }
            }
            _ => {
                let dir = self.root.value.as_str();
                buf.push(dir);
                buf.push(default);
            }
        }

        // Prepend RootDir if set (APT's FindFile prepends it to the result)
        let root_dir = self.node("RootDir");
        if let Some(r) = root_dir
            && !r.value.is_empty()
        {
            let path = buf.to_string_lossy();
            let combined = if path.starts_with('/') {
                format!("{}{}", r.value.trim_end_matches('/'), path)
            } else {
                format!("{}/{}", r.value.trim_end_matches('/'), path)
            };
            return fl_normalize(&combined);
        }

        fl_normalize(&buf.to_string_lossy())
    }

    /// Get dir path from key, or fallback to default
    pub fn get_dir(&self, key: &str, default: &str) -> String {
        let mut path = self.get_file(key, default);
        if !path.ends_with('/') {
            path.push('/');
        }
        path
    }

    /// Get bool from key, or fallback to default
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

    /// Get tree node from key is exist
    pub fn exists(&self, key: &str) -> bool {
        self.node(key).is_some_and(|n| !n.value.is_empty())
    }

    /// Set key and value
    pub fn set(&mut self, key: &str, value: &str) {
        let mut cur = &mut self.root;
        for part in key.split("::") {
            if part == "Dir" {
                continue;
            }
            cur = cur
                .children
                .entry(part.to_string())
                .or_insert_with(Node::empty);
        }
        cur.value = value.to_string();
    }

    /// Set key and list values
    pub fn set_list(&mut self, key: &str, value: &str) {
        let key = key.strip_suffix("::").unwrap_or(key);
        let mut cur = &mut self.root;
        for part in key.split("::") {
            if part == "Dir" {
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

    /// Get tree node from key
    fn node(&self, key: &str) -> Option<&Node> {
        let mut cur = &self.root;
        for part in key.split("::") {
            if part == "Dir" && cur.value == self.root.value {
                continue;
            }
            cur = cur.children.get(part)?;
        }
        Some(cur)
    }
    /// Return the child key names directly under `key`.
    pub fn keys_under(&self, key: &str) -> Vec<String> {
        self.node(key)
            .map(|n| n.children.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// The architectures to read package indexes for — `APT::Architectures`
    /// when configured, otherwise the host architecture
    /// (`APT::Architecture`). `binary-all` is added by the index generator,
    /// like apt.
    ///
    /// No architecture is assumed: if neither is configured, an empty list
    /// is returned (the caller then reads no per-architecture indexes).
    pub fn architectures(&self) -> Vec<String> {
        let archs = self.keys_under("APT::Architectures");
        if archs.is_empty() {
            let native = self.get("APT::Architecture", "");
            if native.is_empty() {
                Vec::new()
            } else {
                vec![native]
            }
        } else {
            archs
        }
    }

    /// For each tree, get full path
    ///
    /// e.g:
    /// Dir -> "/"
    /// Dir::State -> "var/lib/apt"
    /// Dir::State::lists -> "lists/"
    /// Dir::State::lists to Dir::State to Dir:
    /// reverse `lists/ -> var/lib/apt -> /` :
    /// /var/lib/apt/lists
    fn ancestors(&self, key: &str) -> Vec<&str> {
        let mut vals: Vec<&str> = Vec::new();
        let mut cur = &self.root;

        if !cur.value.is_empty() {
            vals.push(cur.value.as_str());
        }

        for part in key.split("::") {
            if part == "Dir" {
                continue;
            }

            match cur.children.get(part) {
                Some(c) => {
                    if !c.value.is_empty() {
                        vals.push(c.value.as_str());
                    }
                    cur = c;
                }
                None => break,
            }
        }

        vals
    }
}

impl Default for AptConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize a path using `canonicalize()` (handles symlinks when path exists),
/// falling back to lexical `Path::components()` resolution for non-existent paths.
fn fl_normalize(path: &str) -> String {
    if path.is_empty() {
        return path.to_string();
    }

    // Try realpath first — handles symlinks, `..`, and `.` when path exists
    if let Ok(canonical) = std::fs::canonicalize(path) {
        let s = canonical.to_string_lossy().to_string();
        if s.starts_with("/dev/null") {
            return String::new();
        }
        return s;
    }

    // Fallback: lexical normalize for non-existent paths
    let p = Path::new(path);
    let mut buf = PathBuf::new();

    for component in p.components() {
        match component {
            Component::CurDir => { /* skip `.` */ }
            Component::ParentDir => {
                match buf.components().next_back() {
                    Some(Component::Normal(_)) => {
                        buf.pop();
                    }
                    Some(Component::RootDir) => {} // root's parent is root
                    _ => {
                        buf.push(Component::ParentDir.as_os_str());
                    }
                }
            }
            other => buf.push(other.as_os_str()),
        }
    }

    let s = buf.to_string_lossy().to_string();
    if s.starts_with("/dev/null") {
        return String::new();
    }

    s
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
        // `..` is resolved lexically (like realpath, no filesystem)
        assert_eq!(fl_normalize("/../foo"), "/foo");
        assert_eq!(fl_normalize("../foo/bar"), "../foo/bar");
        assert_eq!(fl_normalize("foo/../bar"), "bar");
        // /dev/null special case
        assert_eq!(fl_normalize("/dev/null"), "");
        // /dev/../dev/null resolves to /dev/null → cleared
        assert_eq!(fl_normalize("/dev/../dev/null"), "");
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

    #[test]
    fn test_set_and_get() {
        let mut cfg = AptConfig::new();
        cfg.set("APT::Color", "1");
        assert_eq!(cfg.get("APT::Color", ""), "1");
    }

    #[test]
    fn test_set_overrides() {
        let mut cfg = AptConfig::new();
        cfg.set("APT::Color", "1");
        assert_eq!(cfg.get("APT::Color", ""), "1");
        cfg.set("APT::Color", "0");
        assert_eq!(cfg.get("APT::Color", ""), "0");
    }

    #[test]
    fn test_set_nested_key() {
        let mut cfg = AptConfig::new();
        cfg.set("Dir::Etc::sourcelist", "/custom/sources.list");
        assert_eq!(cfg.get("Dir::Etc::sourcelist", ""), "/custom/sources.list");
    }

    #[test]
    fn test_set_skips_dir_segment() {
        let mut cfg = AptConfig::new();
        cfg.set("Dir::State::status", "/custom/status");
        // "Dir" is skipped at root level, so the effective key is State::status
        assert_eq!(cfg.get("Dir::State::status", ""), "/custom/status");
    }

    #[test]
    fn test_set_default_fallback() {
        let cfg = AptConfig::new();
        assert_eq!(cfg.get("Nonexistent::Key", "default"), "default");
    }

    #[test]
    fn test_set_list() {
        let mut cfg = AptConfig::new();
        cfg.set_list("Dir::List", "first");
        cfg.set_list("Dir::List", "second");
        // Each list value becomes a child node with an empty parent value
        assert_eq!(cfg.get("Dir::List", ""), "");
        assert!(cfg.exists("Dir::List::first"));
        assert!(cfg.exists("Dir::List::second"));
    }

    #[test]
    fn test_set_list_with_trailing_colons() {
        let mut cfg = AptConfig::new();
        cfg.set_list("Dir::List::", "item");
        assert!(cfg.exists("Dir::List::item"));
    }

    #[test]
    fn test_set_with_trailing_colons_get() {
        let mut cfg = AptConfig::new();
        // set with trailing :: creates an empty-string child node
        cfg.set("APT::List::", "value");
        assert_eq!(cfg.get("APT::List::", ""), "value");
    }

    #[test]
    fn test_get_with_trailing_colons_non_list() {
        let mut cfg = AptConfig::new();
        // regular set, then get with trailing ::
        cfg.set("APT::Color", "true");
        // get with trailing :: should not find the value (empty-string child doesn't exist)
        assert_eq!(cfg.get("APT::Color::", "fallback"), "fallback");
    }

    #[test]
    fn test_set_list_get_with_trailing_colons() {
        let mut cfg = AptConfig::new();
        cfg.set_list("APT::List", "a");
        cfg.set_list("APT::List", "b");
        // get with trailing :: finds the empty-string child value (which is "")
        assert_eq!(cfg.get("APT::List::", ""), "");
    }
}
