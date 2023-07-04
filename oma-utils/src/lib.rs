use std::{fmt, process::Command};

use number_prefix::NumberPrefix;

#[derive(Debug, thiserror::Error)]
pub enum DpkgArchError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error("`dpkg' returned an error: {0}")]
    DpkgRunError(i32),
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
}

pub fn dpkg_arch() -> Result<String, DpkgArchError> {
    let dpkg = Command::new("dpkg").arg("--print-architecture").output()?;

    if !dpkg.status.success() {
        return Err(DpkgArchError::DpkgRunError(dpkg.status.code().unwrap_or(1)));
    }

    let output = std::str::from_utf8(&dpkg.stdout)?.trim().to_string();

    Ok(output)
}

/// Formats bytes for human readability
#[derive(Debug)]
pub struct HumanBytes(pub u64);

impl fmt::Display for HumanBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match NumberPrefix::binary(self.0 as f64) {
            NumberPrefix::Standalone(number) => write!(f, "{number:.0}B"),
            NumberPrefix::Prefixed(prefix, number) => write!(f, "{number:.2} {prefix}B"),
        }
    }
}
