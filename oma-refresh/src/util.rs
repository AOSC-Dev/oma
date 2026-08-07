use std::{fmt::Display, io, sync::LazyLock};

use oma_apt_pkg::AptListFilename;

use crate::db::RefreshError;

static CVT: LazyLock<AptListFilename> = LazyLock::new(AptListFilename::new);

/// Convert a full repository URL or host+path into an APT list filename.
pub(crate) fn url_to_list_filename(url: &str) -> Result<String, RefreshError> {
    CVT.encode(url)
        .map_err(|e| RefreshError::ReplaceAll(io::Error::other(e.to_string())))
}

#[inline]
pub(crate) fn concat_url_only_check_once_slash(url: &str, path: impl Display) -> String {
    if url.ends_with('/') {
        format!("{url}{path}")
    } else {
        format!("{url}/{path}")
    }
}
