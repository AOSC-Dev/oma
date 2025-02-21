use std::{
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::bail;
use oma_apt_sources_lists::Signature;
use sequoia_openpgp::{
    Cert, KeyHandle,
    cert::CertParser,
    parse::{
        PacketParserBuilder, Parse,
        stream::{
            DetachedVerifierBuilder, MessageLayer, MessageStructure, VerificationError,
            VerificationHelper, VerifierBuilder,
        },
    },
    policy::{AsymmetricAlgorithm, StandardPolicy},
    types::HashAlgorithm,
};
use tracing::debug;

#[derive(Debug)]
pub struct InReleaseVerifier {
    certs: Vec<Cert>,
    trusted: bool,
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
    pub fn from_paths<P: AsRef<Path>>(cert_paths: &[P], trusted: bool) -> VerifyResult<Self> {
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

        Ok(InReleaseVerifier { certs, trusted })
    }

    pub fn from_key_block(block: &str, trusted: bool) -> VerifyResult<Self> {
        // 这个点存在只是表示换行，因此把它替换掉
        let block = block.replace('.', "");
        let mut certs: Vec<Cert> = Vec::new();
        let ppr = PacketParserBuilder::from_bytes(block.as_bytes())?.build()?;
        let cert = CertParser::from(ppr);

        for maybe_cert in cert {
            certs.push(maybe_cert.map_err(|e| VerifyError::BadCertFile(block.to_string(), e))?);
        }

        Ok(InReleaseVerifier { certs, trusted })
    }
}

impl VerificationHelper for InReleaseVerifier {
    fn get_certs(&mut self, _ids: &[KeyHandle]) -> anyhow::Result<Vec<Cert>> {
        Ok(self.certs.clone())
    }

    fn check(&mut self, structure: MessageStructure) -> anyhow::Result<()> {
        if self.trusted {
            return Ok(());
        }

        let mut has_success = false;
        let mut err = None;
        let mut missing_key_err = None;
        for layer in structure {
            if let MessageLayer::SignatureGroup { results } = layer {
                for r in results {
                    match r {
                        Ok(_) => has_success = true,
                        Err(e) => {
                            debug!("{e}");
                            match e {
                                VerificationError::MissingKey { .. } => {
                                    missing_key_err = Some(e);
                                }
                                _ => {
                                    err = Some(e);
                                }
                            }
                        }
                    }
                }
            } else {
                bail!("Malformed PGP signature, InRelease must be signed.")
            }
        }

        if let Some(e) = err {
            bail!("InRelease contains bad signature: {e}.");
        }

        if !has_success {
            bail!(
                "InRelease contains bad signature: {}.",
                missing_key_err.unwrap()
            );
        }

        Ok(())
    }
}

/// Verify InRelease PGP signature
pub fn verify_inrelease(
    inrelease: &str,
    signed_by: Option<&Signature>,
    rootfs: impl AsRef<Path>,
    trusted: bool,
) -> VerifyResult<String> {
    debug!("signed_by: {:?}", signed_by);

    let (certs, deb822_inner_signed_by_str) = find_certs(rootfs, signed_by)?;

    let p = policy();

    let mut v = VerifierBuilder::from_bytes(inrelease.as_bytes())?.with_policy(
        &p,
        None,
        if let Some(deb822_inner_signed_by_str) = deb822_inner_signed_by_str {
            InReleaseVerifier::from_key_block(deb822_inner_signed_by_str, trusted)?
        } else {
            InReleaseVerifier::from_paths(&certs, trusted)?
        },
    )?;

    let mut res = String::new();
    v.read_to_string(&mut res)
        .map_err(VerifyError::FailedToReadInRelease)?;

    Ok(res)
}

fn policy() -> StandardPolicy<'static> {
    // Derive p to allow configuring sequoia_openpgp's StandardPolicy.
    let mut p = StandardPolicy::new();
    // Allow SHA-1 (considering it safe, whereas sequoia_openpgp's standard
    // policy forbids it), as many third party APT repositories still uses
    // SHA-1 to sign their repository metadata (such as InRelease).
    p.accept_hash(HashAlgorithm::SHA1);

    // Allow RSA-1024
    p.accept_asymmetric_algo(AsymmetricAlgorithm::RSA1024);

    p
}

pub fn verify_release(
    release: &str,
    detached: &[u8],
    signed_by: Option<&Signature>,
    rootfs: impl AsRef<Path>,
    trusted: bool,
) -> VerifyResult<()> {
    let (certs, _) = find_certs(rootfs, signed_by)?;
    let p = policy();

    let mut v = DetachedVerifierBuilder::from_bytes(detached)?.with_policy(
        &p,
        None,
        InReleaseVerifier::from_paths(&certs, trusted)?,
    )?;

    v.verify_bytes(release)?;

    Ok(())
}

fn find_certs(
    rootfs: impl AsRef<Path>,
    signed_by: Option<&Signature>,
) -> VerifyResult<(Vec<PathBuf>, Option<&str>)> {
    let rootfs = rootfs.as_ref();

    let mut dir = std::fs::read_dir(rootfs.join("etc/apt/trusted.gpg.d"))
        .map_err(|_| VerifyError::TrustedDirNotExist)?
        .collect::<Vec<_>>();

    let etc_keyring = std::fs::read_dir(rootfs.join("etc/apt/keyrings"));

    if let Ok(keyring) = etc_keyring {
        dir.extend(keyring);
    }

    let mut certs = vec![];
    let mut deb822_inner_signed_by_str = None;

    if let Some(signed_by) = signed_by {
        match signed_by {
            Signature::KeyBlock(block) => {
                deb822_inner_signed_by_str = Some(block.as_str());
                debug!(deb822_inner_signed_by_str);
            }
            Signature::KeyPath(paths) => {
                for p in paths {
                    if p.is_absolute() {
                        certs.push(p.to_path_buf());
                    } else {
                        certs.push(rootfs.join("etc/apt/trusted.gpg.d").join(p))
                    }
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

    Ok((certs, deb822_inner_signed_by_str))
}
