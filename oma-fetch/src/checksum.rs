use faster_hex::{hex_decode, hex_string};
use md5::Md5;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use snafu::ResultExt;
use std::{fmt::Display, fs::File, io, path::Path};

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum Checksum {
    Sha256(Vec<u8>),
    Sha512(Vec<u8>),
    Md5(Vec<u8>),
}

#[derive(Clone, Debug)]
pub enum ChecksumValidator {
    Sha256((Vec<u8>, Sha256)),
    Sha512((Vec<u8>, Sha512)),
    Md5((Vec<u8>, Md5)),
}

impl ChecksumValidator {
    pub fn update(&mut self, data: impl AsRef<[u8]>) {
        match self {
            ChecksumValidator::Sha256((_, v)) => v.update(data),
            ChecksumValidator::Sha512((_, v)) => v.update(data),
            ChecksumValidator::Md5((_, v)) => v.update(data),
        }
    }

    pub fn finish(&self) -> bool {
        match self {
            ChecksumValidator::Sha256((c, v)) => c == &v.clone().finalize().to_vec(),
            ChecksumValidator::Sha512((c, v)) => c == &v.clone().finalize().to_vec(),
            ChecksumValidator::Md5((c, v)) => c == &v.clone().finalize().to_vec(),
        }
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum ChecksumError {
    #[snafu(display("Failed to open file"))]
    OpenFile { source: io::Error, path: Box<Path> },
    #[snafu(display("Failed to checksum file"))]
    Copy { source: io::Error },
    #[snafu(display("Bad Length"))]
    BadLength,
    #[snafu(display("Failed to verify data"))]
    Decode { source: faster_hex::Error },
}

pub type Result<T> = std::result::Result<T, ChecksumError>;

impl Checksum {
    pub fn from_file_sha256(path: &Path) -> Result<Self> {
        let mut file = File::open(path).context(OpenFileSnafu {
            path: Box::from(path),
        })?;

        let mut hasher = Sha256::new();
        io::copy(&mut file, &mut hasher).context(CopySnafu)?;
        let hash = hasher.finalize().to_vec();

        Ok(Self::Sha256(hash))
    }

    /// This function does not do input sanitization, so do checks before!
    pub fn from_sha256_str(s: &str) -> Result<Self> {
        if s.len() != 64 {
            return Err(ChecksumError::BadLength);
        }

        let from = s.as_bytes();
        // dst 的长度必须是 src 的一半
        let mut dst = vec![0; from.len() / 2];
        hex_decode(from, &mut dst).context(DecodeSnafu)?;

        Ok(Checksum::Sha256(dst))
    }

    /// This function does not do input sanitization, so do checks before!
    pub fn from_sha512_str(s: &str) -> Result<Self> {
        if s.len() != 128 {
            return Err(ChecksumError::BadLength);
        }

        let from = s.as_bytes();
        // dst 的长度必须是 src 的一半
        let mut dst = vec![0; from.len() / 2];
        hex_decode(from, &mut dst).context(DecodeSnafu)?;

        Ok(Checksum::Sha512(dst))
    }

    /// This function does not do input sanitization, so do checks before!
    pub fn from_md5_str(s: &str) -> Result<Self> {
        if s.len() != 32 {
            return Err(ChecksumError::BadLength);
        }

        let from = s.as_bytes();
        // dst 的长度必须是 src 的一半
        let mut dst = vec![0; from.len() / 2];
        hex_decode(from, &mut dst).context(DecodeSnafu)?;

        Ok(Checksum::Md5(dst))
    }

    pub fn get_validator(&self) -> ChecksumValidator {
        match self {
            Checksum::Sha256(c) => ChecksumValidator::Sha256((c.clone(), Sha256::new())),
            Checksum::Sha512(c) => ChecksumValidator::Sha512((c.clone(), Sha512::new())),
            Checksum::Md5(c) => ChecksumValidator::Md5((c.clone(), Md5::new())),
        }
    }

    pub fn cmp_read(&self, mut r: Box<dyn std::io::Read>) -> Result<bool> {
        match self {
            Checksum::Sha256(hex) => {
                let mut hasher = Sha256::new();
                io::copy(&mut r, &mut hasher).context(CopySnafu)?;
                let hash = hasher.finalize().to_vec();
                Ok(hex == &hash)
            }
            Checksum::Sha512(hex) => {
                let mut hasher = Sha512::new();
                io::copy(&mut r, &mut hasher).context(CopySnafu)?;
                let hash = hasher.finalize().to_vec();
                Ok(hex == &hash)
            }
            Checksum::Md5(hex) => {
                let mut hasher = Md5::new();
                io::copy(&mut r, &mut hasher).context(CopySnafu)?;
                let hash = hasher.finalize().to_vec();
                Ok(hex == &hash)
            }
        }
    }

    pub fn cmp_file(&self, path: &Path) -> Result<bool> {
        let file = File::open(path).context(OpenFileSnafu {
            path: Box::from(path),
        })?;

        self.cmp_read(Box::new(file) as Box<dyn std::io::Read>)
    }
}

impl Display for Checksum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Checksum::Sha256(hex) => {
                f.write_str("sha256::")?;
                f.write_str(&hex_string(hex))
            }
            Checksum::Sha512(hex) => {
                f.write_str("sha512::")?;
                f.write_str(&hex_string(hex))
            }
            Checksum::Md5(hex) => {
                f.write_str("md5::")?;
                f.write_str(&hex_string(hex))
            }
        }
    }
}
