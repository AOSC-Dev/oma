use std::path::Path;
use std::process::Command;

use tree_sitter::Parser;
use tree_sitter_language::LanguageFn;

use crate::AptConfig;

// Tree-sitter grammar generated from apt-config-grammar/grammar.js
unsafe extern "C" {
    fn tree_sitter_apt_config() -> *const ();
}

const MAX_INCLUDE_DEPTH: usize = 10;

impl AptConfig {
    /// Read a single APT configuration file.
    pub fn load_file(&mut self, path: &str) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.parse_config(&content, 0);

        Ok(())
    }

    /// Read `.conf` files and files without extension from a directory,
    /// sorted alphabetically (like APT's `ReadConfigDir`).
    pub fn load_dir(&mut self, path: &str) -> std::io::Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let name = name.to_string_lossy();
                if name.starts_with('.') {
                    return false;
                }
                // Allow .conf files or files without extension
                name.ends_with(".conf") || !name.contains('.')
            })
            .collect();

        entries.sort_by_key(|e| e.file_name());

        for entry in &entries {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                self.parse_config(&content, 0);
            }
        }

        Ok(())
    }

    /// Load from system default paths: `apt.conf.d/` then `apt.conf`.
    pub fn load_system(&mut self) -> std::io::Result<()> {
        let parts = self.get_dir("Dir::Etc::parts", "etc/apt/apt.conf.d");
        let main = self.get_file("Dir::Etc::main", "etc/apt/apt.conf");

        if Path::new(&parts).is_dir() {
            self.load_dir(&parts)?;
        }

        if Path::new(&main).is_file() {
            self.load_file(&main)?;
        }

        Ok(())
    }

    /// Parse an APT config string using tree-sitter.
    fn parse_config(&mut self, content: &str, depth: usize) {
        if depth > MAX_INCLUDE_DEPTH {
            return;
        }

        let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_apt_config) };
        let language = tree_sitter::Language::new(language_fn);

        let mut parser = Parser::new();
        if parser.set_language(&language).is_err() {
            return;
        }

        let Some(tree) = parser.parse(content, None) else {
            return;
        };

        let root = tree.root_node();
        self.walk_tree(content, root, "", depth);
    }

    /// Walk a tree-sitter CST node and apply config entries.
    fn walk_tree(
        &mut self,
        content: &str,
        node: tree_sitter::Node,
        parent_key: &str,
        depth: usize,
    ) {
        for i in 0..node.child_count() {
            let Some(child) = node.child(i) else {
                continue;
            };
            if !child.is_named() {
                continue;
            }

            match child.kind() {
                "key_value" => self.handle_key_value(content, child, parent_key),
                "list_value" => self.handle_list_value(content, child, parent_key),
                "scope" => self.handle_scope(content, child, parent_key, depth),
                "include_directive" => self.handle_include_directive(content, child, depth),
                "clear_directive" => self.handle_clear_directive(content, child),
                // unknown_statement (# comment), ERROR, MISNG → skip
                _ => {}
            }
        }
    }

    fn handle_key_value(
        &mut self,
        content: &str,
        node: tree_sitter::Node,
        parent_key: &str,
    ) {
        let Some(key_node) = node.child_by_field_name("key") else {
            return;
        };
        let key = node_text(content, key_node).to_string();

        let value: String = {
            let mut cursor = node.walk();
            node.children_by_field_name("value", &mut cursor)
                .map(|n| unescape_string(&node_text(content, n)))
                .collect::<Vec<_>>()
                .join(" ")
        };

        let full_key = if parent_key.is_empty() {
            key
        } else {
            format!("{parent_key}::{key}")
        };

        self.set(&full_key, &value);
    }

    fn handle_list_value(&mut self, content: &str, node: tree_sitter::Node, parent_key: &str) {
        let Some(value_node) = node.child_by_field_name("value") else {
            return;
        };
        let value = unescape_string(&node_text(content, value_node));

        let key = if parent_key.is_empty() {
            value.clone()
        } else {
            format!("{parent_key}::{value}")
        };
        self.set(&key, &value);
    }

    fn handle_scope(
        &mut self,
        content: &str,
        node: tree_sitter::Node,
        parent_key: &str,
        depth: usize,
    ) {
        let Some(key_node) = node.child_by_field_name("key") else {
            return;
        };
        let key = node_text(content, key_node).to_string();

        let full_key = if parent_key.is_empty() {
            key
        } else {
            format!("{parent_key}::{key}")
        };

        self.walk_tree(content, node, &full_key, depth);
    }

    fn handle_include_directive(&mut self, content: &str, node: tree_sitter::Node, depth: usize) {
        let Some(path_node) = node.child_by_field_name("path") else {
            return;
        };
        let path = unescape_string(&node_text(content, path_node));

        if path.ends_with('/') {
            if Path::new(&path).is_dir() {
                let _ = self.load_dir(&path);
            }
        } else {
            let abs_path = if path.starts_with('/') {
                path
            } else {
                format!("{}{}", self.get_dir("Dir::Etc", "etc/apt/"), path)
            };
            if let Ok(c) = std::fs::read_to_string(&abs_path) {
                self.parse_config(&c, depth + 1);
            }
        }
    }

    fn handle_clear_directive(&mut self, content: &str, node: tree_sitter::Node) {
        let Some(key_node) = node.child_by_field_name("key") else {
            return;
        };
        let key = node_text(content, key_node).to_string();
        self.clear_inner(&key);
    }

    fn clear_inner(&mut self, key: &str) {
        let parts = key.split("::");
        let mut current = &mut self.root;

        for part in parts {
            if part == "Dir" {
                continue;
            }

            if let Some(child) = current.children.get_mut(part) {
                current = child;
            } else {
                return;
            }
        }

        current.value.clear();
        current.children.clear();
    }
}

// -- Architecture detection ------------------------------------------------

pub(crate) fn detect_arch() -> Result<String, std::io::Error> {
    let out = Command::new("dpkg-architecture")
        .arg("-qDEB_HOST_ARCH")
        .output()?;

    if out.status.success() {
        let arch = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !arch.is_empty() {
            return Ok(arch);
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "could not detect architecture via dpkg-architecture",
    ))
}

// -- Helpers ----------------------------------------------------------------

/// Extract the text content of a tree-sitter node.
fn node_text<'a>(content: &'a str, node: tree_sitter::Node) -> &'a str {
    node.utf8_text(content.as_bytes()).unwrap_or("")
}

/// Unescape a tree-sitter string node (with quotes removed and `\"` etc.)
fn unescape_string(s: &str) -> String {
    let s = s.strip_prefix('"').unwrap_or(s);
    let s = s.strip_suffix('"').unwrap_or(s);

    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some(c) => out.push(c),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use crate::AptConfig;

    #[test]
    fn test_parse_simple_key_value() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(r#"Test::Key "hello";"#, 0);
        assert_eq!(cfg.get("Test::Key", ""), "hello");
    }

    #[test]
    fn test_parse_empty_value() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(r#"Test::Empty "";"#, 0);
        assert_eq!(cfg.get("Test::Empty", ""), "");
    }

    #[test]
    fn test_parse_nested_scope() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(
            r#"
            APT {
                Install-Recommends "true";
            };
            "#,
            0,
        );
        assert_eq!(cfg.get("APT::Install-Recommends", ""), "true");
    }

    #[test]
    fn test_parse_deeply_nested() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(
            r#"
            Acquire::IndexTargets {
                deb::Packages {
                    MetaKey "$(COMPONENT)/binary-$(ARCHITECTURE)/Packages";
                };
            };
            "#,
            0,
        );
        assert_eq!(
            cfg.get("Acquire::IndexTargets::deb::Packages::MetaKey", ""),
            "$(COMPONENT)/binary-$(ARCHITECTURE)/Packages"
        );
    }

    #[test]
    fn test_parse_list_style_value() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(
            r#"
            APT {
                NeverAutoRemove {
                    "^foo";
                    "^bar";
                };
            };
            "#,
            0,
        );
        assert_eq!(cfg.get("APT::NeverAutoRemove::^foo", ""), "^foo");
        assert_eq!(cfg.get("APT::NeverAutoRemove::^bar", ""), "^bar");
    }

    #[test]
    fn test_parse_multiple_scopes() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(
            r#"
            Dir {
                State "var/lib/apt";
                Cache "var/cache/apt";
            };
            APT {
                Install-Recommends "true";
            };
            "#,
            0,
        );
        assert_eq!(cfg.get("Dir::State", ""), "var/lib/apt");
        assert_eq!(cfg.get("Dir::Cache", ""), "var/cache/apt");
        assert_eq!(cfg.get("APT::Install-Recommends", ""), "true");
    }

    #[test]
    fn test_parse_inline_scope() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(
            "Dir::State \"var/lib/apt\";\nDir::Cache \"var/cache/apt\";\n",
            0,
        );
        assert_eq!(cfg.get("Dir::State", ""), "var/lib/apt");
        assert_eq!(cfg.get("Dir::Cache", ""), "var/cache/apt");
    }

    #[test]
    fn test_empty_config() {
        let mut cfg = AptConfig::new();
        cfg.parse_config("", 0);
        assert_eq!(cfg.get("Dir", ""), "/");
    }

    #[test]
    fn test_comment_ignored() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(
            r#"
            // line comment
            /* block comment */
            Key "value";
            "#,
            0,
        );
        assert_eq!(cfg.get("Key", ""), "value");
    }

    #[test]
    fn test_hash_comment_ignored() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(
            r#"
            # this is a hash comment
            Key "value";
            "#,
            0,
        );
        assert_eq!(cfg.get("Key", ""), "value");
    }

    #[test]
    fn test_escape_sequences() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(r#"Key "hello \"world\"";"#, 0);
        assert_eq!(cfg.get("Key", ""), "hello \"world\"");
    }

    #[test]
    fn test_hash_inside_quotes_not_treated_as_directive() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(r##"APT::Key "#test";"##, 0);
        // # inside quotes should be part of the value, not a directive
        assert_eq!(cfg.get("APT::Key", ""), "#test");
    }

    #[test]
    fn test_slashslash_inside_quotes_is_not_comment() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(r#"APT::Key "http://example.com";"#, 0);
        assert_eq!(cfg.get("APT::Key", ""), "http://example.com");
    }

    #[test]
    fn test_star_inside_quotes_is_not_comment() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(r#"APT::Key "/* not a comment */";"#, 0);
        assert_eq!(cfg.get("APT::Key", ""), "/* not a comment */");
    }

    #[test]
    fn test_parse_comment_handling() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(
            r#"
            // line comment
            # hash comment
            /* block comment */
            Key "value"; /* mid-line */ Key2 "v2";
            "#,
            0,
        );
        assert_eq!(cfg.get("Key", ""), "value");
        assert_eq!(cfg.get("Key2", ""), "v2");
    }

    #[test]
    fn test_parse_multiple_values() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(r#"Key "val1" "val2";"#, 0);
        assert_eq!(cfg.get("Key", ""), "val1 val2");
    }

    #[test]
    fn test_clear_directive() {
        let mut cfg = AptConfig::new();
        cfg.set("APT::Test", "value");
        cfg.parse_config("#clear APT::Test\n", 0);
        assert_eq!(cfg.get("APT::Test", "default"), "default");
    }

    #[test]
    fn test_escape_in_quoted_string() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(r#"Key "hello \"world\"";"#, 0);
        assert_eq!(cfg.get("Key", ""), "hello \"world\"");
    }

    #[test]
    fn test_two_scopes() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(
            r#"
            Dir::State "var/lib/apt";
            APT::Install-Recommends "true";
            "#,
            0,
        );
        assert_eq!(cfg.get("Dir::State", ""), "var/lib/apt");
        assert_eq!(cfg.get("APT::Install-Recommends", ""), "true");
    }

    #[test]
    fn test_load_real_file_01autoremove() {
        let mut cfg = AptConfig::new();
        cfg.load_file("tests/fixtures/01autoremove")
            .expect("should load file");
        assert!(cfg.exists("APT::NeverAutoRemove::^firmware-linux.*"));
        assert_eq!(
            cfg.get("APT::NeverAutoRemove::^firmware-linux.*", ""),
            "^firmware-linux.*"
        );
    }

    #[test]
    fn test_load_real_file_50oma() {
        let mut cfg = AptConfig::new();
        cfg.load_file("tests/fixtures/50oma.conf")
            .expect("should load file");
        let tum = "Acquire::IndexTargets::deb::TUM";
        assert_eq!(cfg.get(&format!("{tum}::MetaKey"), ""), "updates.json");
    }

    #[test]
    fn test_01autoremove_all_entries() {
        let mut cfg = AptConfig::new();
        cfg.load_file("tests/fixtures/01autoremove").unwrap();

        // APT { NeverAutoRemove { ... } }
        assert_eq!(
            cfg.get("APT::NeverAutoRemove::^firmware-linux.*", ""),
            "^firmware-linux.*"
        );
        assert_eq!(
            cfg.get("APT::NeverAutoRemove::^linux-firmware$", ""),
            "^linux-firmware$"
        );
        assert_eq!(
            cfg.get(
                "APT::VersionedKernelPackages::linux-image-unsigned",
                ""
            ),
            "linux-image-unsigned"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::kfreebsd-image", ""),
            "kfreebsd-image"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::linux-source", ""),
            "linux-source"
        );

        // APT { Never-MarkAuto-Sections { ... } }
        assert_eq!(
            cfg.get("APT::Never-MarkAuto-Sections::metapackages", ""),
            "metapackages"
        );
        assert_eq!(
            cfg.get("APT::Move-Autobit-Sections::oldlibs", ""),
            "oldlibs"
        );
    }

    #[test]
    fn test_20packagekit_all_entries() {
        let mut cfg = AptConfig::new();
        cfg.load_file("tests/fixtures/20packagekit").unwrap();
        let dpkg = cfg.get(
            "DPkg::Post-Invoke::\
             /usr/bin/test -e /usr/share/dbus-1/system-services/\
             org.freedesktop.PackageKit.service && /usr/bin/test -S \
             /var/run/dbus/system_bus_socket && /usr/bin/test ! -e \
             /run/ostree-booted && /usr/bin/gdbus call --system --dest \
             org.freedesktop.PackageKit --object-path \
             /org/freedesktop/PackageKit --timeout 4 --method \
             org.freedesktop.PackageKit.StateHasChanged cache-update \
             > /dev/null; /bin/echo > /dev/null",
            "",
        );
        assert!(
            dpkg.contains("PackageKit"),
            "DPkg::Post-Invoke list item should contain PackageKit"
        );
    }

    #[test]
    fn test_50appstream_all_entries() {
        let mut cfg = AptConfig::new();
        cfg.load_file("tests/fixtures/50appstream").unwrap();

        let base = "Acquire::IndexTargets::deb::DEP-11";
        assert_eq!(
            cfg.get(&format!("{base}::MetaKey"), ""),
            "$(COMPONENT)/dep11/Components-$(NATIVE_ARCHITECTURE).yml"
        );
        assert_eq!(
            cfg.get(&format!("{base}::ShortDescription"), ""),
            "Components-$(NATIVE_ARCHITECTURE)"
        );
        assert_eq!(
            cfg.get(&format!("{base}::Description"), ""),
            "$(RELEASE)/$(COMPONENT) $(NATIVE_ARCHITECTURE) Components"
        );
        assert_eq!(cfg.get(&format!("{base}::KeepCompressed"), ""), "true");
        assert_eq!(cfg.get(&format!("{base}::KeepCompressedAs"), ""), "gz");

        let base2 = "Acquire::IndexTargets::deb::DEP-11-icons-small";
        assert_eq!(
            cfg.get(&format!("{base2}::MetaKey"), ""),
            "$(COMPONENT)/dep11/icons-48x48.tar"
        );
        let base6 = "Acquire::IndexTargets::deb::DEP-11-icons-large-hidpi";
        assert_eq!(
            cfg.get(&format!("{base6}::MetaKey"), ""),
            "$(COMPONENT)/dep11/icons-128x128@2.tar"
        );
    }

    #[test]
    fn test_50oma_conf_all_entries() {
        let mut cfg = AptConfig::new();
        cfg.load_file("tests/fixtures/50oma.conf").unwrap();

        let tum = "Acquire::IndexTargets::deb::TUM";
        assert_eq!(cfg.get(&format!("{tum}::MetaKey"), ""), "updates.json");
        assert_eq!(
            cfg.get(&format!("{tum}::ShortDescription"), ""),
            "Topic Update Manifest"
        );
        assert_eq!(
            cfg.get(&format!("{tum}::Description"), ""),
            "Topic Update Manifest"
        );
        assert_eq!(
            cfg.get(&format!("{tum}::flatMetaKey"), ""),
            "Topic Update Manifest"
        );
        assert_eq!(
            cfg.get(&format!("{tum}::flatDescription"), ""),
            "Topic Update Manifest"
        );
        assert_eq!(cfg.get(&format!("{tum}::PDiffs"), ""), "true");
        assert_eq!(cfg.get(&format!("{tum}::KeepCompressed"), ""), "false");

        let cd = "Acquire::IndexTargets::deb::Contents-deb";
        assert_eq!(
            cfg.get(&format!("{cd}::MetaKey"), ""),
            "$(COMPONENT)/Contents-$(ARCHITECTURE)"
        );
        assert_eq!(
            cfg.get(&format!("{cd}::Description"), ""),
            "$(RELEASE)/$(COMPONENT) $(ARCHITECTURE) Contents (deb)"
        );
        assert_eq!(
            cfg.get(&format!("{cd}::flatDescription"), ""),
            "$(RELEASE) Contents (deb)"
        );
        assert_eq!(cfg.get(&format!("{cd}::PDiffs"), ""), "true");
        assert_eq!(cfg.get(&format!("{cd}::KeepCompressed"), ""), "true");

        let dsc = "Acquire::IndexTargets::deb-src::Contents-dsc";
        assert_eq!(
            cfg.get(&format!("{dsc}::MetaKey"), ""),
            "$(COMPONENT)/Contents-source"
        );
        assert_eq!(
            cfg.get(&format!("{dsc}::DefaultEnabled"), ""),
            "false"
        );
    }

    #[test]
    fn test_custom_conf_all_entries() {
        let mut cfg = AptConfig::new();
        cfg.load_file("tests/fixtures/custom.conf").unwrap();
        assert_eq!(cfg.get("Simple", ""), "value");
        assert_eq!(cfg.get("Nested::Key", ""), "value");
        assert_eq!(cfg.get("WithComment::Key", ""), "value");
    }

    #[test]
    fn test_load_dir_all_files() {
        let mut cfg = AptConfig::new();
        cfg.load_dir("tests/fixtures").unwrap();
        assert_eq!(
            cfg.get("APT::NeverAutoRemove::^linux-firmware$", ""),
            "^linux-firmware$"
        );
        assert_eq!(
            cfg.get("Acquire::IndexTargets::deb::TUM::MetaKey", ""),
            "updates.json"
        );
        assert_eq!(cfg.get("Simple", ""), "value");
    }

    #[test]
    fn test_compare_with_apt_config_dump() {
        let dump = std::fs::read_to_string("tests/fixtures/apt-config.dump")
            .expect("apt-config.dump not found");

        let mut cfg = AptConfig::new();
        cfg.init_defaults().unwrap();
        let _ = cfg.load_dir("tests/fixtures");

        // Keys where our hardcoded defaults differ from the running APT
        let known_diffs = [
            "Dir::State::status",
            "APT::Architecture",
            "Acquire::Languages",
            "Dir::Bin::methods",
            "Dir::Bin::dpkg",
            "Dir::Bin::bzip2",
            "Dir::Bin::xz",
            "Dir::Bin::lz4",
            "Dir::Bin::zstd",
            "Dir::Bin::lzma",
            "Dir::Bin::gzip",
            "Dir::Bin::solvers",
            "Dir::Bin::planners",
        ];

        // Skip these prefixes (APT internals / system-specific)
        let skip_prefixes = [
            "Version::",
            "CommandLine::",
            "DPkg::Path",
            "Binary",
            "Acquire::CompressionTypes",
            "Acquire::Changelogs",
            "Acquire::Snapshots",
            "Dir::Ignore-Files-Silently",
            "Dir::Bin",
            "Dir::Media",
            "APT::Solver",
            "APT::Compressor",
            "APT::Architectures",
        ];

        let mut mismatches: Vec<String> = Vec::new();

        for line in dump.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            let quote_start = match line.find('"') {
                Some(i) => i,
                None => continue,
            };
            let key_raw = line[..quote_start].trim();
            let value = line[quote_start + 1..].trim_end_matches("\";").to_string();

            if key_raw.starts_with("Version::") || key_raw == "CommandLine::AsString"
            {
                continue;
            }

            let (our_key, expected_value) = if let Some(parent) = key_raw.strip_suffix("::") {
                (format!("{}::{}", parent.trim(), value), value.clone())
            } else {
                (key_raw.to_string(), value)
            };

            if known_diffs.contains(&key_raw)
                || known_diffs.contains(&key_raw.trim_end_matches("::"))
            {
                continue;
            }

            let our_val = cfg.get(&our_key, "\0");
            if our_val == "\0" {
                let skip_missing = !expected_value.is_empty()
                    && (skip_prefixes.iter().any(|s| key_raw.starts_with(s))
                        || key_raw.starts_with("APT::NeverAutoRemove::")
                            && expected_value.contains("linux-kernel"));
                if !expected_value.is_empty() && !skip_missing {
                    mismatches.push(format!("  MISSING {key_raw} = \"{expected_value}\""));
                }
            } else if our_val != expected_value
                && !(expected_value == "1" && our_val == "true")
                && !(expected_value == "0" && our_val == "false")
            {
                mismatches.push(format!(
                    "  {key_raw}: expected \"{expected_value}\", got \"{our_val}\""
                ));
            }
        }

        assert!(
            mismatches.is_empty(),
            "{} difference(s) with apt-config dump:\n{}",
            mismatches.len(),
            mismatches.join("\n")
        );
    }
}
