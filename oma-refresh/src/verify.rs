use std::{io::Read, path::Path, str::FromStr};

use anyhow::bail;
use sequoia_openpgp::{
    cert::CertParser,
    parse::{
        stream::{MessageLayer, MessageStructure, VerificationHelper, VerifierBuilder},
        PacketParserBuilder, Parse,
    },
    policy::{AsymmetricAlgorithm, StandardPolicy},
    types::HashAlgorithm,
    Cert, KeyHandle,
};

#[derive(Debug)]
pub struct InReleaseVerifier {
    certs: Vec<Cert>,
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

impl FromStr for InReleaseVerifier {
    type Err = VerifyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut certs: Vec<Cert> = Vec::new();
        let ppr = PacketParserBuilder::from_bytes(s.as_bytes())?.build()?;
        let cert = CertParser::from(ppr);

        for maybe_cert in cert {
            certs.push(maybe_cert.map_err(|e| VerifyError::BadCertFile(s.to_string(), e))?);
        }

        Ok(InReleaseVerifier { certs })
    }
}

impl InReleaseVerifier {
    pub fn from_paths<P: AsRef<Path>>(cert_paths: &[P]) -> VerifyResult<Self> {
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

        Ok(InReleaseVerifier { certs })
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
pub fn verify<P: AsRef<Path>>(s: &str, signed_by: Option<&str>, rootfs: P) -> VerifyResult<String> {
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

    let mut deb822_inner_signed_by_str = None;
    if let Some(signed_by) = signed_by {
        let signed_by = signed_by.trim();
        if signed_by.starts_with("-----BEGIN PGP PUBLIC KEY BLOCK-----") {
            deb822_inner_signed_by_str = Some(signed_by);
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

    // Allow RSA-1024
    p.accept_asymmetric_algo(AsymmetricAlgorithm::RSA1024);

    let mut v = VerifierBuilder::from_bytes(s.as_bytes())?.with_policy(
        &p,
        None,
        if let Some(deb822_inner_signed_by_str) = deb822_inner_signed_by_str {
            // 这个点存在只是表示换行，因此把它替换掉
            let signed_by_str = deb822_inner_signed_by_str.replace('.', "");
            InReleaseVerifier::from_str(&signed_by_str)?
        } else {
            InReleaseVerifier::from_paths(&certs)?
        },
    )?;

    let mut res = String::new();
    v.read_to_string(&mut res)
        .map_err(VerifyError::FailedToReadInRelease)?;

    Ok(res)
}
