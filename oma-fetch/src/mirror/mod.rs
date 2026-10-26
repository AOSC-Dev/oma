//! Resolving apt's `mirror://` protocol.
//!
//! A mirror source does not name a repository itself: the URI points at a
//! *mirror list* whose lines name the actual repository base URIs. apt's
//! `methods/mirror.cc` treats this as a per-request redirect — every file is
//! matched against the metadata of the request (`Architecture`, `Component`,
//! `Release`, `Suite`, `Codename`, `Language`, `Type`) and the first usable
//! mirror is used, with the remaining matches kept as fallbacks.
//!
//! Accepted URI forms (apt 1.6+):
//!
//! ```text
//! mirror+file:/etc/apt/mirrors.list   local mirror list
//! mirror+http://host/mirrors.list     remote mirror list
//! mirror+https://host/mirrors.list    remote mirror list
//! mirror://host/mirrors.list          same as mirror+http:// (legacy form)
//! ```
//!
//! A mirror list is one entry per line:
//!
//! ```text
//! # comment
//! http://mirror1.example.com/debian/       priority:1 type:index
//! http://mirror2.example.com/debian/       arch:amd64 arch:all
//! file:/srv/local/debian/                  priority:2
//! ```
//!
//! The URI and its tags are separated by a tab; tags are separated by tabs or
//! spaces. `priority` sorts mirrors (lower first, untagged last); mirrors with
//! equal priority are shuffled so that a plain list such as Ubuntu's
//! `mirrors.txt` does not always hit its first entry. The remaining tags are
//! matched against [`MirrorTarget`]:
//!
//! * `arch`       → `Architecture`
//! * `component`  → `Component`
//! * `suite`      → `Suite` *and* `Release`
//! * `codename`   → `Codename` *and* `Release`
//! * `release`    → `Release`
//! * `lang`       → `Language`
//! * `type`       → `Type` (`index` for all index files, the file extension
//!   for package files, e.g. `deb`)
//!
//! A tag that is absent from a mirror entry never filters it out; a tag whose
//! values do not contain the target value does. This is the same rule as apt.
//!
//! Security: a mirror list fetched over the network may only contain
//! `http(s)` mirrors, never `file:` (apt's CVE-2018-0501 fix). A local list
//! may contain both. Unsupported transports (`ftp`, `copy`, nested mirror
//! transports) are skipped with a debug log.
//!
//! Mirror lists are capped at [`MAX_MIRROR_LIST_SIZE`], matching apt's
//! `MaximumSize`. Compressed lists (`.gz`, `.xz`, …) are recognised but not
//! decoded yet and produce [`MirrorError::UnsupportedCompression`].
//!
//! This module does not persist or share mirror lists between calls; callers
//! should cache a [`MirrorList`] by its source URI for the duration of one
//! refresh or one download transaction, and call [`MirrorList::resolve`] once
//! per file (the target metadata differs per file).

mod entry;
mod error;
mod list;
mod load;
mod target;
mod uri;

pub use error::MirrorError;
pub use list::MirrorList;
pub use load::MAX_MIRROR_LIST_SIZE;
pub use target::{MirrorCandidate, MirrorTarget, MirrorTransport, TargetKind};
pub use uri::{MirrorListLocation, parse_mirror_uri};
