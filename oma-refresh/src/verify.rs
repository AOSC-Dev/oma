use std::{io::Read, path::Path};

use anyhow::bail;
use sequoia_openpgp::{
    cert::CertParser,
    parse::{
        stream::{MessageLayer, MessageStructure, VerificationHelper, VerifierBuilder},
        Parse,
    },
    policy::StandardPolicy,
    types::HashAlgorithm,
    Cert, KeyHandle,
};

pub struct InReleaseVerifier {
    certs: Vec<Cert>,
    _mirror: String,
}

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("Can't parse certificate {0}")]
    CertParseFileError(String, anyhow::Error),
    #[error("Cert file is bad: {0}")]
    BadCertFile(String, anyhow::Error),
    #[error("Does not exist: /etc/apt/trusted.gpg.d")]
    TrustedDirNotExist,
    #[error("Failed to read decoded InRelease file: {0}")]
    FailedToReadInRelease(std::io::Error),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

pub type VerifyResult<T> = Result<T, VerifyError>;

impl InReleaseVerifier {
    pub fn from_paths<P: AsRef<Path>>(cert_paths: &[P], mirror: &str) -> VerifyResult<Self> {
        let mut certs: Vec<Cert> = Vec::new();
        for f in cert_paths {
            for maybe_cert in CertParser::from_file(f)
                .map_err(|e| VerifyError::CertParseFileError(f.as_ref().display().to_string(), e))?
            {
                certs.push(
                    maybe_cert.map_err(|e| {
                        VerifyError::BadCertFile(f.as_ref().display().to_string(), e)
                    })?,
                );
            }
        }

        Ok(InReleaseVerifier {
            certs,
            _mirror: mirror.to_string(),
        })
    }

    pub fn from_str(s: &str, mirror: &str) -> VerifyResult<Self> {
        let mut certs: Vec<Cert> = Vec::new();
        let cert = Cert::from_bytes(s.as_bytes())?;
        certs.push(cert);

        Ok(InReleaseVerifier {
            certs,
            _mirror: mirror.to_string(),
        })
    }
}

impl VerificationHelper for InReleaseVerifier {
    fn get_certs(&mut self, _ids: &[KeyHandle]) -> anyhow::Result<Vec<Cert>> {
        Ok(self.certs.clone())
    }

    fn check(&mut self, structure: MessageStructure) -> anyhow::Result<()> {
        for layer in structure {
            if let MessageLayer::SignatureGroup { results } = layer {
                for r in results {
                    if let Err(e) = r {
                        bail!("InRelease contains bad signature: {e}.")
                    }
                }
            } else {
                bail!("Malformed PGP signature, InRelease must be signed.")
            }
        }

        Ok(())
    }
}

/// Verify InRelease PGP signature
pub fn verify<P: AsRef<Path>>(
    s: &str,
    signed_by: Option<&str>,
    mirror: &str,
    rootfs: P,
) -> VerifyResult<String> {
    let rootfs = rootfs.as_ref();
    let mut dir = std::fs::read_dir(rootfs.join("etc/apt/trusted.gpg.d"))
        .map_err(|_| VerifyError::TrustedDirNotExist)?
        .collect::<Vec<_>>();

    let keyring = std::fs::read_dir(rootfs.join("usr/share/keyrings"));
    let etc_keyring = std::fs::read_dir(rootfs.join("etc/apt/keyrings"));

    if let Ok(keyring) = keyring {
        dir.extend(keyring);
    }

    if let Ok(keyring) = etc_keyring {
        dir.extend(keyring);
    }

    let mut certs = vec![];

    let mut inner_signed_by = false;
    if let Some(signed_by) = signed_by {
        if signed_by.starts_with("---BEGIN PGP PUBLIC KEY BLOCK---") {
            inner_signed_by = true;
        } else {
            let trust_files = signed_by.split(',');
            for file in trust_files {
                let p = Path::new(file);
                if p.is_absolute() {
                    certs.push(p.to_path_buf());
                } else {
                    certs.push(rootfs.join("etc/apt/trusted.gpg.d").join(file))
                }
            }
        }
    } else {
        for i in dir.iter().flatten() {
            let path = i.path();
            let ext = path.extension().and_then(|x| x.to_str());
            if ext == Some("gpg") || ext == Some("asc") {
                certs.push(i.path().to_path_buf());
            }
        }

        let trust_main = rootfs.join("etc/apt/trusted.gpg").to_path_buf();

        if trust_main.is_file() {
            certs.push(trust_main);
        }
    }

    // Derive p to allow configuring sequoia_openpgp's StandardPolicy.
    let mut p = StandardPolicy::new();
    // Allow SHA-1 (considering it safe, whereas sequoia_openpgp's standard
    // policy forbids it), as many third party APT repositories still uses
    // SHA-1 to sign their repository metadata (such as InRelease).
    p.accept_hash(HashAlgorithm::SHA1);

    let mut v = VerifierBuilder::from_bytes(s.as_bytes())?.with_policy(
        &p,
        None,
        if inner_signed_by {
            InReleaseVerifier::from_str(signed_by.unwrap(), mirror)?
        } else {
            InReleaseVerifier::from_paths(&certs, mirror)?
        },
    )?;

    let mut res = String::new();
    v.read_to_string(&mut res)
        .map_err(VerifyError::FailedToReadInRelease)?;

    Ok(res)
}
