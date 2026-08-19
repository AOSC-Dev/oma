use std::path::Path;

use deb822_fast::{FromDeb822, FromDeb822Paragraph, ToDeb822, ToDeb822Paragraph};
use deb822_lossless::Deb822;

/// Errors that can occur when reading or writing dpkg status files.
#[derive(Debug, thiserror::Error)]
pub enum DpkgError {
    #[error("Failed to read dpkg status file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse deb822 data: {0}")]
    Deb822(#[from] deb822_lossless::Error),
    #[error("Failed to parse package entry: {0}")]
    Entry(String),
    /// The package has no installed status — like `apt-mark` refusing to
    /// mark a package that is not installed.
    #[error("package {0} is not installed")]
    NotInstalled(String),
    /// A mark or write-back was attempted on a lazily-loaded status
    /// ([`DpkgState::from_file_lazy`](crate::dpkg_state::DpkgState::from_file_lazy)),
    /// which never read the status tree: writing it back would truncate the
    /// status file.
    #[error("dpkg status was loaded lazily; cannot mark packages or write it back")]
    NotLoaded,
}

/// Errors parsing a dpkg `Status` value into a [`PkgStatus`].
#[derive(Debug, thiserror::Error)]
pub enum StatusParseError {
    /// The status value had fewer than three words.
    #[error("missing word in dpkg status")]
    Missing,
    /// The status value had more than three words.
    #[error("malformed dpkg status: {0}")]
    Malformed(String),
    /// A word in the status value was not recognised.
    #[error("unknown dpkg state: {0}")]
    Unknown(String),
}

/// The desired selection state of a package — the first word of the dpkg
/// `Status` field (`install` / `hold` / `deinstall` / `purge`), like apt's
/// `PkgSelectedState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionState {
    Unknown,
    Install,
    Hold,
    Deinstall,
    Purge,
}

impl SelectionState {
    /// Whether the package is selected as installed (install or hold).
    pub fn is_installed(self) -> bool {
        matches!(self, Self::Install | Self::Hold)
    }

    fn parse(word: &str) -> Result<Self, StatusParseError> {
        match word {
            "unknown" => Ok(Self::Unknown),
            "install" => Ok(Self::Install),
            "hold" => Ok(Self::Hold),
            "deinstall" => Ok(Self::Deinstall),
            "purge" => Ok(Self::Purge),
            _ => Err(StatusParseError::Unknown(word.to_string())),
        }
    }

    fn as_word(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Install => "install",
            Self::Hold => "hold",
            Self::Deinstall => "deinstall",
            Self::Purge => "purge",
        }
    }
}

/// The error flag of a package's dpkg `Status` — its second word
/// (`ok` / `reinstreq` / …), like apt's `PkgInstState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstState {
    Ok,
    ReinstReq,
    HoldInst,
    HoldReinstReq,
}

impl InstState {
    fn parse(word: &str) -> Result<Self, StatusParseError> {
        match word {
            "ok" => Ok(Self::Ok),
            "reinstreq" => Ok(Self::ReinstReq),
            "hold" => Ok(Self::HoldInst),
            "hold-reinstreq" => Ok(Self::HoldReinstReq),
            _ => Err(StatusParseError::Unknown(word.to_string())),
        }
    }

    fn as_word(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::ReinstReq => "reinstreq",
            Self::HoldInst => "hold",
            Self::HoldReinstReq => "hold-reinstreq",
        }
    }
}

/// The current state of a package's dpkg `Status` — its third word
/// (`not-installed` / `config-files` / `half-installed` / …), like apt's
/// `PkgCurrentState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentState {
    NotInstalled,
    ConfigFiles,
    HalfInstalled,
    Unpacked,
    HalfConfigured,
    TriggersAwaited,
    TriggersPending,
    Installed,
}

impl CurrentState {
    fn parse(word: &str) -> Result<Self, StatusParseError> {
        match word {
            "not-installed" => Ok(Self::NotInstalled),
            "config-files" => Ok(Self::ConfigFiles),
            "half-installed" => Ok(Self::HalfInstalled),
            "unpacked" => Ok(Self::Unpacked),
            "half-configured" => Ok(Self::HalfConfigured),
            "triggers-awaited" => Ok(Self::TriggersAwaited),
            "triggers-pending" => Ok(Self::TriggersPending),
            "installed" => Ok(Self::Installed),
            _ => Err(StatusParseError::Unknown(word.to_string())),
        }
    }

    fn as_word(self) -> &'static str {
        match self {
            Self::NotInstalled => "not-installed",
            Self::ConfigFiles => "config-files",
            Self::HalfInstalled => "half-installed",
            Self::Unpacked => "unpacked",
            Self::HalfConfigured => "half-configured",
            Self::TriggersAwaited => "triggers-awaited",
            Self::TriggersPending => "triggers-pending",
            Self::Installed => "installed",
        }
    }
}

/// A package's full dpkg `Status` value — `<desired> <error> <current>`,
/// e.g. `hold ok installed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PkgStatus {
    /// Desired selection state (first word).
    pub selection: SelectionState,
    /// Error flag (second word).
    pub error: InstState,
    /// Current state (third word).
    pub current: CurrentState,
}

impl PkgStatus {
    /// `install ok installed` — a healthy installed package.
    pub const INSTALLED: Self = Self {
        selection: SelectionState::Install,
        error: InstState::Ok,
        current: CurrentState::Installed,
    };

    /// `hold ok installed` — a healthy held package.
    pub const HELD: Self = Self {
        selection: SelectionState::Hold,
        error: InstState::Ok,
        current: CurrentState::Installed,
    };

    /// Parse the three-word dpkg `Status` value.
    fn parse(status: &str) -> Result<Self, StatusParseError> {
        let mut words = status.split_whitespace();
        let selection = SelectionState::parse(words.next().ok_or(StatusParseError::Missing)?)?;
        let error = InstState::parse(words.next().ok_or(StatusParseError::Missing)?)?;
        let current = CurrentState::parse(words.next().ok_or(StatusParseError::Missing)?)?;
        if words.next().is_some() {
            return Err(StatusParseError::Malformed(status.to_string()));
        }
        Ok(Self {
            selection,
            error,
            current,
        })
    }

    /// Serialize back to the full dpkg `Status` value.
    fn serialize(&self) -> String {
        format!(
            "{} {} {}",
            self.selection.as_word(),
            self.error.as_word(),
            self.current.as_word()
        )
    }

    /// Whether the package is selected as installed (install or hold).
    pub fn is_installed(&self) -> bool {
        self.selection.is_installed()
    }

    /// Whether the installed package needs reinstalling — selected for
    /// install but with the `reinstreq` error flag, or a current state that
    /// is in-progress / broken (half-installed, unpacked, …).
    pub fn needs_reinstall(&self) -> bool {
        self.selection.is_installed()
            && (matches!(self.error, InstState::ReinstReq | InstState::HoldReinstReq)
                || matches!(
                    self.current,
                    CurrentState::HalfInstalled
                        | CurrentState::Unpacked
                        | CurrentState::HalfConfigured
                        | CurrentState::TriggersAwaited
                        | CurrentState::TriggersPending
                ))
    }
}

/// Parse an `Essential:` / `Protected:` value the way apt reads a deb822 flag
/// — `pkgTagSection::FindFlag` → `StringToBool`
/// (`apt-pkg/contrib/strutl.cc`): `0`/`1` (with the optional `+` `strtol`
/// allows), the `no`/`false`/`without`/`off`/`disable` words, and the
/// `yes`/`true`/`with`/`on`/`enable` words, all case-insensitive. Anything
/// else — an empty value included — reads as false, the state apt leaves the
/// flag in when it does not recognise the value.
///
/// `strtol` also reads `0x1`/`010`; dpkg and apt only ever write `yes`/`no`,
/// so only the plain forms are read here.
pub(crate) fn parse_flag(value: &str) -> Result<bool, String> {
    let value = value.trim();
    match value.strip_prefix('+').unwrap_or(value) {
        "0" => return Ok(false),
        "1" => return Ok(true),
        _ => {}
    }
    if ["no", "false", "without", "off", "disable"]
        .iter()
        .any(|word| value.eq_ignore_ascii_case(word))
    {
        return Ok(false);
    }
    Ok(["yes", "true", "with", "on", "enable"]
        .iter()
        .any(|word| value.eq_ignore_ascii_case(word)))
}

/// Serialize an `Essential`/`Protected` boolean back to `yes`/`no` (the
/// `ToString` default would write `true`/`false`).
fn serialize_yes(value: &bool) -> String {
    if *value {
        "yes".to_string()
    } else {
        "no".to_string()
    }
}

/// Parse the `Status` field into a [`PkgStatus`] (the `FromDeb822` hook —
/// the derive requires a `String` error, so the typed parse error is
/// flattened here).
fn parse_status(value: &str) -> Result<PkgStatus, String> {
    PkgStatus::parse(value).map_err(|e| e.to_string())
}

/// Serialize a [`PkgStatus`] back to the full dpkg `Status` value (the
/// `ToDeb822` hook).
fn serialize_status(value: &PkgStatus) -> String {
    value.serialize()
}

/// Information about a single package from dpkg status.
#[derive(Debug, Clone, FromDeb822, ToDeb822)]
pub struct DpkgPackage {
    #[deb822(field = "Package")]
    pub name: String,
    #[deb822(field = "Version")]
    pub version: Option<String>,
    #[deb822(field = "Architecture")]
    pub architecture: Option<String>,
    /// The package's full dpkg `Status` — `<desired> <error> <current>`
    /// (e.g. `hold ok installed`) — round-tripped via the typed enums.
    #[deb822(field = "Status", deserialize_with = parse_status, serialize_with = serialize_status)]
    pub status: Option<PkgStatus>,
    /// `Essential: yes` — the package is essential; apt refuses to remove it.
    #[deb822(field = "Essential", deserialize_with = parse_flag, serialize_with = serialize_yes)]
    pub essential: Option<bool>,
    /// `Protected: yes` (dpkg 1.19+) — the package is protected; apt refuses
    /// to remove it (apt's internal `Flag::Important`).
    #[deb822(field = "Protected", deserialize_with = parse_flag, serialize_with = serialize_yes)]
    pub protected: Option<bool>,
}

impl DpkgPackage {
    /// The package's desired selection state — [`SelectionState::Unknown`]
    /// when no `Status` field is set.
    pub fn selection_state(&self) -> SelectionState {
        self.status
            .map(|s| s.selection)
            .unwrap_or(SelectionState::Unknown)
    }

    /// Whether the installed package needs reinstalling — selected for
    /// install but with the `reinstreq` error flag or an in-progress /
    /// broken current state (half-installed, unpacked, …).
    pub fn needs_reinstall(&self) -> bool {
        self.status.is_some_and(|s| s.needs_reinstall())
    }
}
/// Read the dpkg status file at `path` into a lossless deb822 tree — the
/// representation [`DpkgState`](crate::dpkg_state::DpkgState) writes back.
pub fn read_status_tree(path: impl AsRef<Path>) -> Result<Deb822, DpkgError> {
    Ok(Deb822::from_file(path)?)
}

/// Extract package entries from an already-loaded status `tree`.
pub fn packages_from_tree(tree: &Deb822) -> Result<Vec<DpkgPackage>, DpkgError> {
    tree.paragraphs()
        .map(|paragraph| DpkgPackage::from_paragraph(&paragraph).map_err(DpkgError::Entry))
        .collect()
}

/// Parse `/var/lib/dpkg/status` and return full package information.
pub fn parse_dpkg_status(path: impl AsRef<Path>) -> Result<Vec<DpkgPackage>, DpkgError> {
    packages_from_tree(&read_status_tree(path)?)
}

/// Set `name`'s desired selection state on the loaded status `tree` — the
/// paragraph is parsed into a typed [`DpkgPackage`] (`FromDeb822`), the
/// `selection` word is assigned (preserving the error/current words, like
/// `dpkg --set-selections`), and written back with `update_paragraph`
/// (`ToDeb822`), in place on the lossless tree (like
/// `tree[pkg].status.selection = SelectionState::Hold`).
///
/// A paragraph that does not parse is skipped, like the query index — one
/// malformed stanza must not block marking the packages that do parse.
///
/// Only installed stanzas are touched. On multiarch systems dpkg status can
/// hold a leftover stanza of another architecture in `deinstall …` /
/// `purge …`; rewriting its selection to `hold` would change dpkg's intent
/// and make the stale stanza count as installed on the next load, so it is
/// left alone. Every installed stanza of `name` is updated, not just the
/// first — the query index ([`DpkgState`]) keys holds by name, so updating
/// only one architecture would leave the tree and the index disagreeing.
pub(crate) fn set_pkg_status(
    tree: &mut Deb822,
    name: &str,
    selection: SelectionState,
) -> Result<(), DpkgError> {
    let mut found = false;
    for mut paragraph in tree.paragraphs() {
        let Ok(mut pkg) = DpkgPackage::from_paragraph(&paragraph) else {
            continue;
        };
        if pkg.name != name || !pkg.selection_state().is_installed() {
            continue;
        }
        let mut status = pkg.status.unwrap_or(PkgStatus::INSTALLED);
        status.selection = selection;
        pkg.status = Some(status);
        pkg.update_paragraph(&mut paragraph);
        found = true;
    }
    if found {
        Ok(())
    } else {
        Err(DpkgError::Entry(format!(
            "package {name} not found in status"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dpkg_status_round_trip() {
        let parse = |status: &str| PkgStatus::parse(status).unwrap();
        assert!(parse("install ok installed").is_installed());
        assert!(parse("hold ok installed").is_installed());
        assert!(!parse("deinstall ok config-files").is_installed());
        assert!(!parse("purge ok not-installed").is_installed());
        assert!(!parse("unknown ok not-installed").is_installed());

        // All three words are preserved, not just the selection.
        let broken = parse("install reinstreq half-installed");
        assert!(broken.is_installed());
        assert!(broken.needs_reinstall());
        assert_eq!(broken.error, InstState::ReinstReq);
        assert_eq!(broken.current, CurrentState::HalfInstalled);
        assert!(parse("install ok unpacked").needs_reinstall());
        assert!(parse("install ok half-configured").needs_reinstall());
        assert!(!parse("install ok installed").needs_reinstall());
        assert!(!parse("deinstall ok config-files").needs_reinstall());

        // Serializes back to the full dpkg Status value.
        assert_eq!(PkgStatus::HELD.serialize(), "hold ok installed");
        assert_eq!(PkgStatus::INSTALLED.serialize(), "install ok installed");
        assert_eq!(broken.serialize(), "install reinstreq half-installed");
        // Malformed statuses are typed errors, not raw strings.
        assert!(matches!(
            PkgStatus::parse("hold ok"),
            Err(StatusParseError::Missing)
        ));
        assert!(matches!(
            PkgStatus::parse("install ok installed extra"),
            Err(StatusParseError::Malformed(_))
        ));
        assert!(matches!(
            PkgStatus::parse("install foo installed"),
            Err(StatusParseError::Unknown(_))
        ));
    }

    #[test]
    fn test_parse_dpkg_status_with_all_fields() {
        let input = "\
Package: zoxide
Version: 0.9.6-1
Architecture: amd64
Status: install ok installed
Essential: yes
Protected: yes

Package: vim
Version: 9.1.0
Architecture: amd64
Status: hold ok installed

Package: old-kernel
Version: 6.0.0
Status: deinstall ok config-files

";
        let dir = std::env::temp_dir().join("test_dpkg_status");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("status");
        std::fs::write(&path, input).unwrap();

        let packages = parse_dpkg_status(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(packages.len(), 3);

        assert_eq!(packages[0].name, "zoxide");
        assert_eq!(packages[0].version.as_deref(), Some("0.9.6-1"));
        assert_eq!(packages[0].architecture.as_deref(), Some("amd64"));
        assert!(packages[0].selection_state().is_installed());
        assert_eq!(packages[0].essential, Some(true));
        assert_eq!(packages[0].protected, Some(true));

        assert_eq!(packages[1].name, "vim");
        assert!(packages[1].selection_state().is_installed());
        assert_eq!(packages[1].essential, None);
        assert_eq!(packages[1].protected, None);

        assert_eq!(packages[2].name, "old-kernel");
        assert!(!packages[2].selection_state().is_installed());
    }

    /// `hold-reinstreq` is the held counterpart of `reinstreq`: dpkg still
    /// records the package as requiring reinstallation.
    #[test]
    fn hold_reinstreq_needs_reinstall() {
        let parse = |status: &str| PkgStatus::parse(status).unwrap();
        assert!(parse("hold hold-reinstreq installed").needs_reinstall());
        assert!(!parse("hold ok installed").needs_reinstall());
    }

    #[test]
    fn parse_flag_follows_apt_string_to_bool() {
        // apt's `StringToBool`: `0`/`1` plus both word lists, case-insensitive.
        for (value, expected) in [
            ("yes", true),
            ("YES", true),
            ("true", true),
            ("with", true),
            ("on", true),
            ("enable", true),
            ("1", true),
            ("+1", true),
            ("no", false),
            ("false", false),
            ("without", false),
            ("off", false),
            ("disable", false),
            ("0", false),
            // Not a value apt recognises — the flag stays unset.
            ("", false),
            ("maybe", false),
            ("2", false),
        ] {
            assert_eq!(parse_flag(value).unwrap(), expected, "value: {value:?}");
        }
    }
}
