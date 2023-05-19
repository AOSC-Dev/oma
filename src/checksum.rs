use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::{fmt::Display, fs::File, io, path::Path};

use crate::fl;

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum Checksum {
    Sha256(Vec<u8>),
    Sha512(Vec<u8>),
}

#[derive(Clone)]
pub enum ChecksumValidator {
    Sha256((Vec<u8>, Sha256)),
    Sha512((Vec<u8>, Sha512)),
}

impl ChecksumValidator {
    pub fn update(&mut self, data: impl AsRef<[u8]>) {
        match self {
            ChecksumValidator::Sha256((_, v)) => v.update(data),
            ChecksumValidator::Sha512((_, v)) => v.update(data),
        }
    }

    pub fn finish(&self) -> bool {
        match self {
            ChecksumValidator::Sha256((c, v)) => c == &v.clone().finalize().to_vec(),
            ChecksumValidator::Sha512((c, v)) => c == &v.clone().finalize().to_vec(),
        }
    }
}

impl Checksum {
    // pub fn from_file_sha256(path: &Path) -> Result<Self> {
    //     let mut file = File::open(path).context(format!(
    //         "Failed to open {} for checking checksum",
    //         path.display()
    //     ))?;
    //     let mut hasher = Sha256::new();
    //     io::copy(&mut file, &mut hasher)?;
    //     let hash = hasher.finalize().to_vec();
    //     Ok(Self::Sha256(hash))
    // }

    /// This function does not do input sanitization, so do checks before!
    pub fn from_sha256_str(s: &str) -> Result<Self> {
        if s.len() != 64 {
            bail!(fl!("sha256-bad-length"))
        }
        Ok(Checksum::Sha256(hex::decode(s)?))
    }

    /// This function does not do input sanitization, so do checks before!
    // pub fn from_sha512_str(s: &str) -> Result<Self> {
    //     if s.len() != 128 {
    //         bail!("Malformed SHA512 checksum: bad length.")
    //     }
    //     Ok(Checksum::Sha512(hex::decode(s)?))
    // }

    pub fn get_validator(&self) -> ChecksumValidator {
        match self {
            Checksum::Sha256(c) => ChecksumValidator::Sha256((c.clone(), Sha256::new())),
            Checksum::Sha512(c) => ChecksumValidator::Sha512((c.clone(), Sha512::new())),
        }
    }

    pub fn cmp_read(&self, mut r: Box<dyn std::io::Read>) -> Result<bool> {
        match self {
            Checksum::Sha256(hex) => {
                let mut hasher = Sha256::new();
                io::copy(&mut r, &mut hasher)
                    .map_err(|e| anyhow!(fl!("can-not-checksum", e = e.to_string())))?;
                let hash = hasher.finalize().to_vec();
                Ok(hex == &hash)
            }
            Checksum::Sha512(hex) => {
                let mut hasher = Sha512::new();
                io::copy(&mut r, &mut hasher)
                    .map_err(|e| anyhow!(fl!("can-not-checksum", e = e.to_string())))?;
                let hash = hasher.finalize().to_vec();
                Ok(hex == &hash)
            }
        }
    }

    pub fn cmp_file(&self, path: &Path) -> Result<bool> {
        let file = File::open(path).context(fl!(
            "failed-to-open-to-checksum",
            path = path.display().to_string()
        ))?;

        self.cmp_read(Box::new(file) as Box<dyn std::io::Read>)
    }
}

impl Display for Checksum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Checksum::Sha256(hex) => {
                f.write_str("sha256::")?;
                f.write_str(&hex::encode(hex))
            }
            Checksum::Sha512(hex) => {
                f.write_str("sha512::")?;
                f.write_str(&hex::encode(hex))
            }
        }
    }
}
