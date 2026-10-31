/// Metadata apt attaches to an acquire request (`Target-*`), used to select
/// mirrors from a [`crate::mirror::MirrorList`].
///
/// Every field is optional: a missing/empty field disables that matcher. Use
/// [`TargetKind::Index`] for every index file (including `InRelease`) and
/// [`TargetKind::File`] for package files, matching apt's `Target-Type`.
#[derive(Debug, Clone, Copy, Default)]
pub struct MirrorTarget<'a> {
    /// `Target-Architecture`, e.g. `amd64` (or `all` for `binary-all`).
    pub arch: Option<&'a str>,
    /// `Target-Component`, e.g. `main`.
    pub component: Option<&'a str>,
    /// `Target-Release`: the suite as written in sources.list.
    pub release: Option<&'a str>,
    /// `Target-Suite`: the `Suite` field of the Release file (package
    /// downloads only).
    pub suite: Option<&'a str>,
    /// `Target-Codename`: the `Codename` field of the Release file (package
    /// downloads only).
    pub codename: Option<&'a str>,
    /// `Target-Language`, e.g. `zh_CN` for translation files.
    pub language: Option<&'a str>,
    /// `Target-Type`.
    pub kind: TargetKind<'a>,
}

/// The `Target-Type` of a request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TargetKind<'a> {
    /// Do not filter by type.
    #[default]
    Unspecified,
    /// An index file (`InRelease`, `Packages.xz`, `Translation-zh_CN.xz`, …).
    Index,
    /// A package file; the value is its extension, e.g. `deb`.
    File(&'a str),
}

impl<'a> TargetKind<'a> {
    pub(super) fn apt_type(self) -> Option<&'a str> {
        match self {
            Self::Unspecified => None,
            Self::Index => Some("index"),
            Self::File(extension) => Some(extension),
        }
    }
}

/// Transport used to reach a mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorTransport {
    Http,
    File,
}

/// One concrete download URL produced from a mirror list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorCandidate {
    /// Full download URL for [`crate::mirror::MirrorList::resolve`].
    pub url: String,
    pub transport: MirrorTransport,
    /// The mirror's `priority:` tag; lower is tried first, `u64::MAX` when
    /// the entry has no explicit priority.
    pub priority: u64,
}
