use std::process::Command;

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

