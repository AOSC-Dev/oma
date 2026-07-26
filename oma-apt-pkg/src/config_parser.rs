use std::process::Command;

use crate::AptConfig;

impl AptConfig {
    /// Read a single APT configuration file.
    pub fn load_file(&mut self, path: &str) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.parse_config(&content);
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
                self.parse_config(&content);
            }
        }
        Ok(())
    }

    /// Load from system default paths: `apt.conf.d/` then `apt.conf`.
    pub fn load_system(&mut self) -> std::io::Result<()> {
        let parts = self.get_dir("Dir::Etc::parts", "etc/apt/apt.conf.d");
        let main = self.get_file("Dir::Etc::main", "etc/apt/apt.conf");
        if std::path::Path::new(&parts).is_dir() {
            self.load_dir(&parts)?;
        }
        if std::path::Path::new(&main).is_file() {
            self.load_file(&main)?;
        }
        Ok(())
    }

    fn parse_config(&mut self, content: &str) {
        let b = content.as_bytes();
        let len = b.len();
        let mut i = 0;
        let mut stack: Vec<String> = Vec::new();
        let mut parent = String::new();

        while i < len {
            i = skip(b, i);
            if i >= len {
                break;
            }

            // #include / #clear directives
            if b[i] == b'#' {
                i = self.dir(b, i);
                continue;
            }

            // Read key name (or list value inside a scope)
            let (key, next) = ident(b, i);
            if key.is_empty() {
                if b[i] == b'"' {
                    i = self.read_value(b, i, &parent);
                    continue;
                }
                // Let {, }, ; be handled by the match below
                if b[i] == b'{' || b[i] == b'}' || b[i] == b';' {
                    // fall through to match
                } else {
                    i = next + 1;
                    continue;
                }
            }
            i = skip(b, next);
            if i >= len {
                break;
            }

            match b[i] {
                b'{' => {
                    i += 1;
                    let full = if parent.is_empty() {
                        key
                    } else {
                        format!("{}::{}", parent, key)
                    };
                    stack.push(parent.clone());
                    parent = full;
                }
                b'}' => {
                    i += 1;
                    parent = stack.pop().unwrap_or_default();
                    i = skip(b, i);
                    if i < len && b[i] == b';' {
                        i += 1;
                    }
                }
                _ => {
                    // Read value(s)
                    let mut vals: Vec<String> = Vec::new();
                    loop {
                        i = skip(b, i);
                        if i >= len || b[i] == b';' || b[i] == b'}' || b[i] == b'{' {
                            break;
                        }
                        if b[i] == b'"' {
                            if let Some((v, n)) = quoted(b, i) {
                                vals.push(v);
                                i = n;
                                continue;
                            }
                            // Unterminated quote — skip the " and continue
                            i += 1;
                            continue;
                        }
                        break;
                    }
                    i = skip(b, i);
                    if i < len && b[i] == b';' {
                        i += 1;
                    }

                    let full = if parent.is_empty() {
                        key
                    } else {
                        format!("{}::{}", parent, key)
                    };
                    self.set(&full, &vals.join(" "));
                }
            }
        }
    }

    /// Read a value (or list of values) under `parent` scope, terminated by `;`.
    fn read_value(&mut self, b: &[u8], mut i: usize, parent: &str) -> usize {
        let mut vals: Vec<String> = Vec::new();
        loop {
            i = skip(b, i);
            if i >= b.len() || b[i] == b';' || b[i] == b'}' || b[i] == b'{' {
                break;
            }
            if b[i] == b'"' {
                if let Some((v, n)) = quoted(b, i) {
                    vals.push(v);
                    i = n;
                    continue;
                }
                // Unterminated quote — skip the " and continue
                i += 1;
                continue;
            }
            break;
        }
        i = skip(b, i);
        if i < b.len() && b[i] == b';' {
            i += 1;
        }
        let value = vals.join(" ");
        if !value.is_empty() {
            let key = if parent.is_empty() {
                value.clone()
            } else {
                format!("{}::{}", parent, value)
            };
            self.set(&key, &value);
        }
        i
    }

    /// Handle #include / #clear directives.
    fn dir(&mut self, b: &[u8], mut i: usize) -> usize {
        let start = i;
        while i < b.len() && b[i] != b'\n' {
            i += 1;
        }
        let line = std::str::from_utf8(&b[start..i]).unwrap_or("");
        let rest = line.trim_start_matches('#').trim();

        if let Some(file) = rest.strip_prefix("include ") {
            let file = file.trim().trim_matches('"');
            let path = if file.starts_with('/') {
                file.to_string()
            } else {
                format!("{}{}", self.get_dir("Dir::Etc", "etc/apt"), file)
            };
            if let Ok(c) = std::fs::read_to_string(&path) {
                self.parse_config(&c);
            }
        } else if let Some(key) = rest.strip_prefix("clear ") {
            self.clear(key.trim());
        }

        i + 1
    }

    fn clear(&mut self, key: &str) {
        let parts: Vec<&str> = key.split("::").collect();
        let mut current = &mut self.root;
        for part in &parts {
            if *part == "Dir" {
                continue;
            }
            if let Some(child) = current.children.get_mut(*part) {
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

// -- Byte-level parser helpers --------------------------------------------

/// Skip whitespace and block comments (//, /* */).
/// Does NOT skip `#` so the main loop can handle #include/#clear.
fn skip(b: &[u8], mut i: usize) -> usize {
    let len = b.len();
    while i < len {
        match b[i] {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'/' if i + 1 < len && b[i + 1] == b'/' => {
                i += 2;
                while i < len && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < len && b[i + 1] == b'*' => {
                i += 2;
                while i + 1 < len && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
            }
            _ => break,
        }
    }
    i
}

/// Read an identifier (alphanumeric + `::`, `_`, `-`, `+`, `.`).
fn ident(b: &[u8], i: usize) -> (String, usize) {
    let mut j = i;
    while j < b.len()
        && (b[j].is_ascii_alphanumeric() || matches!(b[j], b':' | b'_' | b'-' | b'+' | b'.'))
    {
        j += 1;
    }
    if j > i {
        (String::from_utf8_lossy(&b[i..j]).to_string(), j)
    } else {
        (String::new(), i)
    }
}

/// Read a double-quoted string (without quotes), handling `\\` escapes.
fn quoted(b: &[u8], i: usize) -> Option<(String, usize)> {
    if i >= b.len() || b[i] != b'"' {
        return None;
    }
    let mut j = i + 1;
    let mut s = String::new();
    while j < b.len() {
        if b[j] == b'\\' && j + 1 < b.len() {
            s.push(b[j + 1] as char);
            j += 2;
        } else if b[j] == b'"' {
            return Some((s, j + 1));
        } else {
            s.push(b[j] as char);
            j += 1;
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::AptConfig;

    #[test]
    fn test_parse_simple_key_value() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(r#"Test::Key "hello";"#);
        assert_eq!(cfg.get("Test::Key", ""), "hello");
    }

    #[test]
    fn test_parse_empty_value() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(r#"Test::Empty ";"#);
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
                    "^firmware-linux.*";
                    "^linux-firmware$";
                };
            };
            "#,
        );
        assert!(cfg.exists("APT::NeverAutoRemove::^firmware-linux.*"));
        assert_eq!(
            cfg.get("APT::NeverAutoRemove::^firmware-linux.*", ""),
            "^firmware-linux.*"
        );
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
        );
        assert_eq!(cfg.get("Key", ""), "value");
        assert_eq!(cfg.get("Key2", ""), "v2");
    }

    #[test]
    fn test_parse_multiple_values() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(r#"Key "val1" "val2";"#);
        assert_eq!(cfg.get("Key", ""), "val1 val2");
    }

    #[test]
    fn test_load_real_file_01autoremove() {
        let mut cfg = AptConfig::new();
        cfg.load_file("tests/fixtures/01autoremove")
            .expect("should load file");
        // Items inside NeverAutoRemove scope should be stored
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
        assert_eq!(
            cfg.get("Acquire::IndexTargets::deb::TUM::MetaKey", ""),
            "updates.json"
        );
    }

    #[test]
    fn test_clear_directive() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(
            r#"
            Test::Key "value";
            #clear Test
            "#,
        );
        // After #clear, the key should no longer exist
        assert_eq!(cfg.get("Test::Key", "default"), "default");
    }

    #[test]
    fn test_two_scopes() {
        let mut cfg = AptConfig::new();
        // Test: flat keys with scopes - no nesting
        cfg.parse_config(
            r#"
            APT::Scope1 { "item1"; };
            APT::Scope2 { "item2"; };
        "#,
        );
        assert!(cfg.exists("APT::Scope1::item1"), "Scope1 item missing");
        assert!(cfg.exists("APT::Scope2::item2"), "Scope2 item missing");
    }
    #[test]
    fn test_escape_in_quoted_string() {
        let mut cfg = AptConfig::new();
        cfg.parse_config(r#"Key "hello\"world";"#);
        // \" inside a quoted string produces a literal "
        assert_eq!(cfg.get("Key", ""), "hello\"world");
    }
    #[test]
    fn test_01autoremove_all_entries() {
        let mut cfg = AptConfig::new();
        cfg.load_file("tests/fixtures/01autoremove").unwrap();

        // Debug: check all keys in tree
        let never_key = "APT::NeverAutoRemove::^firmware-linux.*";
        let vk_key = "APT::VersionedKernelPackages::linux-image";
        eprintln!(
            "NeverAutoRemove exists: {} val={:?}",
            cfg.exists(never_key),
            cfg.get(never_key, "")
        );
        eprintln!(
            "VersionedKernelPackages exists: {} val={:?}",
            cfg.exists(vk_key),
            cfg.get(vk_key, "")
        );
        // Check parent scope
        eprintln!(
            "APT::VersionedKernelPackages exists: {} val={:?}",
            cfg.exists("APT::VersionedKernelPackages"),
            cfg.get("APT::VersionedKernelPackages", "")
        );

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
            cfg.get("APT::NeverAutoRemove::^linux-image-[a-z0-9]*$", ""),
            "^linux-image-[a-z0-9]*$"
        );
        assert_eq!(
            cfg.get(
                "APT::NeverAutoRemove::^linux-image-[a-z0-9]*-[a-z0-9]*$",
                ""
            ),
            "^linux-image-[a-z0-9]*-[a-z0-9]*$"
        );

        // APT { VersionedKernelPackages { ... } }
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::linux-image", ""),
            "linux-image"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::linux-headers", ""),
            "linux-headers"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::linux-image-extra", ""),
            "linux-image-extra"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::linux-modules", ""),
            "linux-modules"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::linux-modules-extra", ""),
            "linux-modules-extra"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::linux-signed-image", ""),
            "linux-signed-image"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::linux-image-unsigned", ""),
            "linux-image-unsigned"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::kfreebsd-image", ""),
            "kfreebsd-image"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::kfreebsd-headers", ""),
            "kfreebsd-headers"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::gnumach-image", ""),
            "gnumach-image"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::.*-modules", ""),
            ".*-modules"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::.*-kernel", ""),
            ".*-kernel"
        );
        assert_eq!(
            cfg.get(
                "APT::VersionedKernelPackages::linux-backports-modules-.*",
                ""
            ),
            "linux-backports-modules-.*"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::linux-modules-.*", ""),
            "linux-modules-.*"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::linux-tools", ""),
            "linux-tools"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::linux-cloud-tools", ""),
            "linux-cloud-tools"
        );
        assert_eq!(
            cfg.get("APT::VersionedKernelPackages::linux-buildinfo", ""),
            "linux-buildinfo"
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
            cfg.get("APT::Never-MarkAuto-Sections::contrib/metapackages", ""),
            "contrib/metapackages"
        );
        assert_eq!(
            cfg.get("APT::Never-MarkAuto-Sections::non-free/metapackages", ""),
            "non-free/metapackages"
        );
        assert_eq!(
            cfg.get("APT::Never-MarkAuto-Sections::restricted/metapackages", ""),
            "restricted/metapackages"
        );
        assert_eq!(
            cfg.get("APT::Never-MarkAuto-Sections::universe/metapackages", ""),
            "universe/metapackages"
        );
        assert_eq!(
            cfg.get("APT::Never-MarkAuto-Sections::multiverse/metapackages", ""),
            "multiverse/metapackages"
        );

        // APT { Move-Autobit-Sections { ... } }
        assert_eq!(
            cfg.get("APT::Move-Autobit-Sections::oldlibs", ""),
            "oldlibs"
        );
        assert_eq!(
            cfg.get("APT::Move-Autobit-Sections::contrib/oldlibs", ""),
            "contrib/oldlibs"
        );
        assert_eq!(
            cfg.get("APT::Move-Autobit-Sections::non-free/oldlibs", ""),
            "non-free/oldlibs"
        );
        assert_eq!(
            cfg.get("APT::Move-Autobit-Sections::restricted/oldlibs", ""),
            "restricted/oldlibs"
        );
        assert_eq!(
            cfg.get("APT::Move-Autobit-Sections::universe/oldlibs", ""),
            "universe/oldlibs"
        );
        assert_eq!(
            cfg.get("APT::Move-Autobit-Sections::multiverse/oldlibs", ""),
            "multiverse/oldlibs"
        );
    }

    #[test]
    fn test_20packagekit_all_entries() {
        let mut cfg = AptConfig::new();
        cfg.load_file("tests/fixtures/20packagekit").unwrap();

        // DPkg::Post-Invoke is a list: the command is a list item
        // The list item value becomes the leaf key under the parent scope
        let dpkg = cfg.get("DPkg::Post-Invoke::/usr/bin/test -e /usr/share/dbus-1/system-services/org.freedesktop.PackageKit.service && /usr/bin/test -S /var/run/dbus/system_bus_socket && /usr/bin/test ! -e /run/ostree-booted && /usr/bin/gdbus call --system --dest org.freedesktop.PackageKit --object-path /org/freedesktop/PackageKit --timeout 4 --method org.freedesktop.PackageKit.StateHasChanged cache-update > /dev/null; /bin/echo > /dev/null", "");
        assert!(
            dpkg.contains("PackageKit"),
            "DPkg::Post-Invoke list item should contain PackageKit"
        );

        // APT::Update::Post-Invoke-Success { "long command"; }
        let update = cfg.get("APT::Update::Post-Invoke-Success::/usr/bin/test -e /usr/share/dbus-1/system-services/org.freedesktop.PackageKit.service && /usr/bin/test -S /var/run/dbus/system_bus_socket && /usr/bin/test ! -e /run/ostree-booted && /usr/bin/gdbus call --system --dest org.freedesktop.PackageKit --object-path /org/freedesktop/PackageKit --timeout 4 --method org.freedesktop.PackageKit.StateHasChanged cache-update > /dev/null; /bin/echo > /dev/null", "");
        assert!(
            update.contains("PackageKit"),
            "APT::Update::Post-Invoke-Success list item should contain PackageKit"
        );
    }

    #[test]
    fn test_50appstream_all_entries() {
        let mut cfg = AptConfig::new();
        cfg.load_file("tests/fixtures/50appstream").unwrap();

        // Acquire::IndexTargets { deb::DEP-11 { ... } }
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

        // deb::DEP-11-icons-small
        let base2 = "Acquire::IndexTargets::deb::DEP-11-icons-small";
        assert_eq!(
            cfg.get(&format!("{base2}::MetaKey"), ""),
            "$(COMPONENT)/dep11/icons-48x48.tar"
        );

        // deb::DEP-11-icons
        let base3 = "Acquire::IndexTargets::deb::DEP-11-icons";
        assert_eq!(
            cfg.get(&format!("{base3}::MetaKey"), ""),
            "$(COMPONENT)/dep11/icons-64x64.tar"
        );

        // deb::DEP-11-icons-hidpi
        let base4 = "Acquire::IndexTargets::deb::DEP-11-icons-hidpi";
        assert_eq!(
            cfg.get(&format!("{base4}::MetaKey"), ""),
            "$(COMPONENT)/dep11/icons-64x64@2.tar"
        );

        // deb::DEP-11-icons-large
        let base5 = "Acquire::IndexTargets::deb::DEP-11-icons-large";
        assert_eq!(
            cfg.get(&format!("{base5}::MetaKey"), ""),
            "$(COMPONENT)/dep11/icons-128x128.tar"
        );

        // deb::DEP-11-icons-large-hidpi
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

        // Acquire::IndexTargets { deb::TUM { ... } }
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

        // deb::Contents-deb
        let cd = "Acquire::IndexTargets::deb::Contents-deb";
        assert_eq!(
            cfg.get(&format!("{cd}::MetaKey"), ""),
            "$(COMPONENT)/Contents-$(ARCHITECTURE)"
        );
        assert_eq!(
            cfg.get(&format!("{cd}::ShortDescription"), ""),
            "Contents-$(ARCHITECTURE)"
        );
        assert_eq!(
            cfg.get(&format!("{cd}::Description"), ""),
            "$(RELEASE)/$(COMPONENT) $(ARCHITECTURE) Contents (deb)"
        );
        assert_eq!(
            cfg.get(&format!("{cd}::flatMetaKey"), ""),
            "Contents-$(ARCHITECTURE)"
        );
        assert_eq!(
            cfg.get(&format!("{cd}::flatDescription"), ""),
            "$(RELEASE) Contents (deb)"
        );
        assert_eq!(cfg.get(&format!("{cd}::PDiffs"), ""), "true");
        assert_eq!(cfg.get(&format!("{cd}::KeepCompressed"), ""), "true");

        // deb::BinContents-deb
        let bcd = "Acquire::IndexTargets::deb::BinContents-deb";
        assert_eq!(
            cfg.get(&format!("{bcd}::MetaKey"), ""),
            "$(COMPONENT)/BinContents-$(ARCHITECTURE)"
        );
        assert_eq!(
            cfg.get(&format!("{bcd}::ShortDescription"), ""),
            "BinContents-$(ARCHITECTURE)"
        );
        assert_eq!(
            cfg.get(&format!("{bcd}::Description"), ""),
            "$(RELEASE)/$(COMPONENT) $(ARCHITECTURE) BinContents (deb)"
        );
        assert_eq!(
            cfg.get(&format!("{bcd}::flatMetaKey"), ""),
            "BinContents-$(ARCHITECTURE)"
        );
        assert_eq!(
            cfg.get(&format!("{bcd}::flatDescription"), ""),
            "$(RELEASE) BinContents (deb)"
        );
        assert_eq!(cfg.get(&format!("{bcd}::PDiffs"), ""), "true");
        assert_eq!(cfg.get(&format!("{bcd}::KeepCompressed"), ""), "false");

        // deb-src::Contents-dsc
        let dsc = "Acquire::IndexTargets::deb-src::Contents-dsc";
        assert_eq!(
            cfg.get(&format!("{dsc}::MetaKey"), ""),
            "$(COMPONENT)/Contents-source"
        );
        assert_eq!(
            cfg.get(&format!("{dsc}::ShortDescription"), ""),
            "Contents-source"
        );
        assert_eq!(
            cfg.get(&format!("{dsc}::Description"), ""),
            "$(RELEASE)/$(COMPONENT) source Contents (dsc)"
        );
        assert_eq!(
            cfg.get(&format!("{dsc}::flatMetaKey"), ""),
            "Contents-source"
        );
        assert_eq!(
            cfg.get(&format!("{dsc}::flatDescription"), ""),
            "$(RELEASE) Contents (dsc)"
        );
        assert_eq!(cfg.get(&format!("{dsc}::PDiffs"), ""), "true");
        assert_eq!(cfg.get(&format!("{dsc}::KeepCompressed"), ""), "true");
        assert_eq!(cfg.get(&format!("{dsc}::DefaultEnabled"), ""), "false");

        // deb::Contents-udeb
        let udeb = "Acquire::IndexTargets::deb::Contents-udeb";
        assert_eq!(
            cfg.get(&format!("{udeb}::MetaKey"), ""),
            "$(COMPONENT)/Contents-udeb-$(ARCHITECTURE)"
        );
        assert_eq!(
            cfg.get(&format!("{udeb}::ShortDescription"), ""),
            "Contents-udeb-$(ARCHITECTURE)"
        );
        assert_eq!(
            cfg.get(&format!("{udeb}::Description"), ""),
            "$(RELEASE)/$(COMPONENT) $(ARCHITECTURE) Contents (udeb)"
        );
        assert_eq!(
            cfg.get(&format!("{udeb}::flatMetaKey"), ""),
            "Contents-udeb-$(ARCHITECTURE)"
        );
        assert_eq!(
            cfg.get(&format!("{udeb}::flatDescription"), ""),
            "$(RELEASE) Contents (udeb)"
        );
        assert_eq!(cfg.get(&format!("{udeb}::KeepCompressed"), ""), "true");
        assert_eq!(cfg.get(&format!("{udeb}::PDiffs"), ""), "true");
        assert_eq!(cfg.get(&format!("{udeb}::DefaultEnabled"), ""), "false");

        // deb::Contents-deb-legacy (FALLBACKS — no trailing ; on close brace)
        let legacy = "Acquire::IndexTargets::deb::Contents-deb-legacy";
        assert_eq!(
            cfg.get(&format!("{legacy}::MetaKey"), ""),
            "Contents-$(ARCHITECTURE)"
        );
        assert_eq!(
            cfg.get(&format!("{legacy}::ShortDescription"), ""),
            "Contents-$(ARCHITECTURE)"
        );
        assert_eq!(
            cfg.get(&format!("{legacy}::Description"), ""),
            "$(RELEASE) $(ARCHITECTURE) Contents (deb)"
        );
        assert_eq!(cfg.get(&format!("{legacy}::PDiffs"), ""), "true");
        assert_eq!(cfg.get(&format!("{legacy}::KeepCompressed"), ""), "true");
        assert_eq!(
            cfg.get(&format!("{legacy}::Fallback-Of"), ""),
            "Contents-deb"
        );
        assert_eq!(
            cfg.get(&format!("{legacy}::Identifier"), ""),
            "Contents-deb"
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

        // From 01autoremove (no extension)
        assert_eq!(
            cfg.get("APT::NeverAutoRemove::^linux-firmware$", ""),
            "^linux-firmware$"
        );

        // From 50oma.conf
        assert_eq!(
            cfg.get("Acquire::IndexTargets::deb::TUM::MetaKey", ""),
            "updates.json"
        );

        // From custom.conf
        assert_eq!(cfg.get("Simple", ""), "value");
    }

    #[test]
    fn test_compare_with_apt_config_dump() {
        let dump = std::fs::read_to_string("tests/fixtures/apt-config.dump").expect(
            "apt-config.dump not found — run `apt-config dump > tests/fixtures/apt-config.dump`",
        );

        let mut cfg = AptConfig::new();
        cfg.init_defaults().unwrap();
        // Load the fixture config files (same set used to generate the dump)
        let _ = cfg.load_dir("tests/fixtures");

        // Keys where our hardcoded defaults may differ from the running APT
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

        // Skip these prefixes entirely (APT internals / system-specific)
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

            // Skip system-specific keys not in our defaults
            if key_raw.starts_with("Version::") || key_raw == "CommandLine::AsString" {
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
                // Skip missing entries that come from system config files
                // not loaded (e.g. 01autoremove-kernels is runtime-generated)
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
