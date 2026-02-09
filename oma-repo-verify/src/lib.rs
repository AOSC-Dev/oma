use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::SystemTime,
};

use anyhow::bail;
use chrono::{DateTime, NaiveDate, Utc};
use oma_apt_sources_lists::Signature;
use sequoia_openpgp::{
    Cert, KeyHandle,
    cert::CertParser,
    packet::Tag,
    parse::{
        PacketParserBuilder, Parse,
        stream::{
            DetachedVerifierBuilder, MessageLayer, MessageStructure, VerificationError,
            VerificationHelper, VerifierBuilder,
        },
    },
    policy::{AsymmetricAlgorithm, HashAlgoSecurity, StandardPolicy},
    types::HashAlgorithm,
};
use sequoia_policy_config::ConfiguredStandardPolicy;
use spdlog::{debug, warn};

static POLICY: OnceLock<StandardPolicy<'static>> = OnceLock::new();

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
pub fn verify_inrelease_by_sysroot(
    inrelease: &str,
    signed_by: Option<&Signature>,
    rootfs: impl AsRef<Path>,
    trusted: bool,
) -> VerifyResult<String> {
    debug!("signed_by: {:?}", signed_by);

    let kob = find_certs(rootfs, signed_by)?;
    let res = verify_inrelease_inner(inrelease, trusted, kob)?;

    Ok(res)
}

pub fn verify_inrelease_inner(
    inrelease: &str,
    trusted: bool,
    kob: KeyBlockOrPaths<'_>,
) -> Result<String, VerifyError> {
    let p = POLICY.get_or_init(policy);

    let mut v = VerifierBuilder::from_bytes(inrelease.as_bytes())?.with_policy(
        p,
        None,
        match kob {
            KeyBlockOrPaths::Block(block) => InReleaseVerifier::from_key_block(block, trusted)?,
            KeyBlockOrPaths::Paths(certs) => InReleaseVerifier::from_paths(&certs, trusted)?,
        },
    )?;

    let mut res = String::new();
    v.read_to_string(&mut res)
        .map_err(VerifyError::FailedToReadInRelease)?;

    Ok(res)
}

fn policy() -> StandardPolicy<'static> {
    let mut p = ConfiguredStandardPolicy::new();
    let default_configure_path = Path::new("/usr/share/apt/default-sequoia.config");
    let custom_configure_path = Path::new("/etc/crypto-policies/back-ends/apt-sequoia.config");

    for path in [custom_configure_path, default_configure_path] {
        match p.parse_config_file(path) {
            Ok(true) => {
                debug!("config file {} loaded", path.display());
                return p.build();
            }
            Ok(false) => {
                debug!("config file {} does not exist", path.display());
            }
            Err(e) => {
                warn!("Failed to parse config file {}: {e}", path.display());
            }
        }
    }

    debug!("No config file found, using built-in policy");
    default_policy()
}

// From Debian apt default policy:
// https://salsa.debian.org/apt-team/apt/-/blob/main/debian/default-sequoia.config
fn default_policy() -> StandardPolicy<'static> {
    let mut policy = StandardPolicy::new();

    let expire_20240201: SystemTime = DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(2024, 2, 1).unwrap().into(),
        Utc,
    )
    .into();

    let expire_20280201: SystemTime = DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(2028, 2, 1).unwrap().into(),
        Utc,
    )
    .into();

    let expire_20300201: SystemTime = DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(2030, 2, 1).unwrap().into(),
        Utc,
    )
    .into();

    let expire_20260201: SystemTime = DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(2026, 2, 1).unwrap().into(),
        Utc,
    )
    .into();

    policy.reject_asymmetric_algo_at(AsymmetricAlgorithm::DSA2048, Some(expire_20240201));
    policy.reject_asymmetric_algo_at(AsymmetricAlgorithm::DSA3072, Some(expire_20240201));
    policy.reject_asymmetric_algo_at(AsymmetricAlgorithm::DSA4096, Some(expire_20240201));
    policy.reject_asymmetric_algo_at(AsymmetricAlgorithm::BrainpoolP256, Some(expire_20280201));
    policy.reject_asymmetric_algo_at(AsymmetricAlgorithm::BrainpoolP384, Some(expire_20280201));
    policy.reject_asymmetric_algo_at(AsymmetricAlgorithm::BrainpoolP512, Some(expire_20280201));
    policy.reject_asymmetric_algo_at(AsymmetricAlgorithm::RSA2048, Some(expire_20300201));

    policy.reject_hash_property_at(
        HashAlgorithm::SHA1,
        HashAlgoSecurity::SecondPreImageResistance,
        Some(expire_20260201),
    );

    policy.reject_hash_at(HashAlgorithm::SHA224, Some(expire_20260201));
    policy.reject_packet_tag_version_at(Tag::Signature, 3, Some(expire_20260201));

    policy
}

pub fn verify_release_by_sysroot(
    release: &str,
    detached: &[u8],
    signed_by: Option<&Signature>,
    rootfs: impl AsRef<Path>,
    trusted: bool,
) -> VerifyResult<()> {
    let kob = find_certs(rootfs, signed_by)?;
    verify_release_inner(release, detached, trusted, kob)?;

    Ok(())
}

pub fn verify_release_inner(
    release: &str,
    detached: &[u8],
    trusted: bool,
    bop: KeyBlockOrPaths<'_>,
) -> Result<(), VerifyError> {
    let p = POLICY.get_or_init(policy);

    let mut v = DetachedVerifierBuilder::from_bytes(detached)?.with_policy(
        p,
        None,
        match bop {
            KeyBlockOrPaths::Block(block) => InReleaseVerifier::from_key_block(block, trusted)?,
            KeyBlockOrPaths::Paths(certs) => InReleaseVerifier::from_paths(&certs, trusted)?,
        },
    )?;

    v.verify_bytes(release)?;

    Ok(())
}

pub enum KeyBlockOrPaths<'a> {
    Block(&'a str),
    Paths(Vec<PathBuf>),
}

fn find_certs(
    rootfs: impl AsRef<Path>,
    signed_by: Option<&Signature>,
) -> VerifyResult<KeyBlockOrPaths<'_>> {
    let rootfs = rootfs.as_ref();

    let mut certs = vec![];

    if let Some(signed_by) = signed_by {
        match signed_by {
            Signature::KeyBlock(block) => {
                return Ok(KeyBlockOrPaths::Block(block));
            }
            Signature::KeyPath(paths) => {
                if paths.is_empty() {
                    certs = find_default_dir_certs(rootfs)?;
                }

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
        certs = find_default_dir_certs(rootfs)?;
    }

    Ok(KeyBlockOrPaths::Paths(certs))
}

fn find_default_dir_certs(rootfs: &Path) -> Result<Vec<PathBuf>, VerifyError> {
    let mut certs = vec![];
    let mut dir = std::fs::read_dir(rootfs.join("etc/apt/trusted.gpg.d"))
        .map_err(|_| VerifyError::TrustedDirNotExist)?
        .collect::<Vec<_>>();
    let etc_keyring = std::fs::read_dir(rootfs.join("etc/apt/keyrings"));

    if let Ok(keyring) = etc_keyring {
        dir.extend(keyring);
    }

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

    Ok(certs)
}
