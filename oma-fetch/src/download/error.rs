//! Error types for the download pipeline, plus their (de)serialization.

use std::io;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::SingleDownloadErrorHelper;

#[derive(Debug, Error)]
pub enum BuilderError {
    #[error("Download task {file_name} sources is empty")]
    EmptySource { file_name: String },
    #[error("Not allow set illegal download threads: {count}")]
    IllegalDownloadThread { count: usize },
}

#[derive(Debug, Error)]
pub enum SingleDownloadError {
    #[error("Failed to open file")]
    Open(io::Error),
    #[error("Failed to create file")]
    Create(io::Error),
    #[error("Failed to seek file")]
    Seek(io::Error),
    #[error("Failed to write file")]
    Write(io::Error),
    #[error("Failed to flush file")]
    Flush(io::Error),
    #[error("Failed to remove file")]
    Remove(io::Error),
    #[error("Failed to create symlink")]
    CreateSymlink(io::Error),
    #[error("Request Error")]
    ReqwestMiddlewareError(reqwest_middleware::Error),
    #[error("Failed to read")]
    Read(io::Error),
    #[error("Send request timeout")]
    SendRequestTimeout,
    #[error("Download file timeout")]
    DownloadTimeout,
    #[error("checksum mismatch")]
    ChecksumMismatch,
    #[error("semaphore acquire error")]
    AcquireError,
}

impl Serialize for SingleDownloadError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let helper = match self {
            Self::Open(source) => SingleDownloadErrorHelper::Open {
                source: source.to_string(),
            },
            Self::Create(source) => SingleDownloadErrorHelper::Create {
                source: source.to_string(),
            },
            Self::Seek(source) => SingleDownloadErrorHelper::Seek {
                source: source.to_string(),
            },
            Self::Write(source) => SingleDownloadErrorHelper::Write {
                source: source.to_string(),
            },
            Self::Flush(source) => SingleDownloadErrorHelper::Flush {
                source: source.to_string(),
            },
            Self::Remove(source) => SingleDownloadErrorHelper::Remove {
                source: source.to_string(),
            },
            Self::CreateSymlink(source) => SingleDownloadErrorHelper::CreateSymlink {
                source: source.to_string(),
            },
            Self::ReqwestMiddlewareError(source) => {
                SingleDownloadErrorHelper::ReqwestMiddlewareError {
                    source: source.to_string(),
                }
            }
            Self::Read(source) => SingleDownloadErrorHelper::Read {
                source: source.to_string(),
            },
            Self::SendRequestTimeout => SingleDownloadErrorHelper::SendRequestTimeout,
            Self::DownloadTimeout => SingleDownloadErrorHelper::DownloadTimeout,
            Self::ChecksumMismatch => SingleDownloadErrorHelper::ChecksumMismatch,
            Self::AcquireError => SingleDownloadErrorHelper::AcquireError,
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SingleDownloadError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = SingleDownloadErrorHelper::deserialize(deserializer)?;

        let error = match helper {
            SingleDownloadErrorHelper::Open { source } => Self::Open(io::Error::other(source)),
            SingleDownloadErrorHelper::Create { source } => Self::Create(io::Error::other(source)),
            SingleDownloadErrorHelper::Seek { source } => Self::Seek(io::Error::other(source)),
            SingleDownloadErrorHelper::Write { source } => Self::Write(io::Error::other(source)),
            SingleDownloadErrorHelper::Flush { source } => Self::Flush(io::Error::other(source)),
            SingleDownloadErrorHelper::Remove { source } => Self::Remove(io::Error::other(source)),
            SingleDownloadErrorHelper::CreateSymlink { source } => {
                Self::CreateSymlink(io::Error::other(source))
            }

            // reqwest_middleware::Error 无法简单 new，但它通常支持从标准 Error 转换，或者转为自定义格式
            // 这里可以通过 anyhow 或标准映射转换为中间状态错误
            SingleDownloadErrorHelper::ReqwestMiddlewareError { source } => {
                Self::ReqwestMiddlewareError(reqwest_middleware::Error::Middleware(
                    anyhow::anyhow!(source),
                ))
            }

            SingleDownloadErrorHelper::Read { source } => Self::Read(io::Error::other(source)),
            SingleDownloadErrorHelper::SendRequestTimeout => Self::SendRequestTimeout,
            SingleDownloadErrorHelper::DownloadTimeout => Self::DownloadTimeout,
            SingleDownloadErrorHelper::ChecksumMismatch => Self::ChecksumMismatch,
            SingleDownloadErrorHelper::AcquireError => Self::AcquireError,
        };

        Ok(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip_all_variants() {
        let cases = vec![
            SingleDownloadError::Open(io::Error::other("open")),
            SingleDownloadError::Create(io::Error::other("create")),
            SingleDownloadError::Seek(io::Error::other("seek")),
            SingleDownloadError::Write(io::Error::other("write")),
            SingleDownloadError::Flush(io::Error::other("flush")),
            SingleDownloadError::Remove(io::Error::other("remove")),
            SingleDownloadError::CreateSymlink(io::Error::other("symlink")),
            SingleDownloadError::ReqwestMiddlewareError(reqwest_middleware::Error::Middleware(
                anyhow::anyhow!("req"),
            )),
            SingleDownloadError::Read(io::Error::other("read")),
            SingleDownloadError::SendRequestTimeout,
            SingleDownloadError::DownloadTimeout,
            SingleDownloadError::ChecksumMismatch,
            SingleDownloadError::AcquireError,
        ];

        for error in cases {
            let json = serde_json::to_string(&error).unwrap();
            let decoded: SingleDownloadError = serde_json::from_str(&json).unwrap();
            assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
        }
    }
}
