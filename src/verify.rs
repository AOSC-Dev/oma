use std::{io::Read, path::Path};

use anyhow::{bail, Context, Result};
use sequoia_openpgp::{
    cert::CertParser,
    parse::{
        stream::{MessageLayer, MessageStructure, VerificationHelper, VerifierBuilder},
        Parse,
    },
    policy::StandardPolicy,
    Cert, KeyHandle,
};

// use crate::due_to;

pub struct InReleaseVerifier {
    certs: Vec<Cert>,
    _mirror: String,
}

impl InReleaseVerifier {
    pub fn new<P: AsRef<Path>>(cert_paths: &[P], mirror: &str) -> Result<Self> {
        let mut certs: Vec<Cert> = Vec::new();
        for f in cert_paths {
            for maybe_cert in CertParser::from_file(f).context(format!(
                "Failed to load certs from file {:?}",
                f.as_ref().display()
            ))? {
                certs.push(maybe_cert.context(format!(
                    "A cert from file {:?} is bad",
                    f.as_ref().display()
                ))?);
            }
        }

        Ok(InReleaseVerifier {
            certs,
            _mirror: mirror.to_string(),
        })
    }
}

impl VerificationHelper for InReleaseVerifier {
    fn get_certs(&mut self, ids: &[KeyHandle]) -> Result<Vec<Cert>> {
        let mut certs = Vec::new();
        for id in ids {
            for cert in &self.certs {
                if &cert.key_handle() == id {
                    certs.push(cert.clone());
                }
            }
        }

        Ok(certs)
    }

    fn check(&mut self, structure: MessageStructure) -> Result<()> {
        for layer in structure {
            if let MessageLayer::SignatureGroup { results } = layer {
                for r in results {
                    if let Err(e) = r {
                        // TODO: 签名失败的时候加上 dueto
                        // match &e {
                        //     // VerificationError::MalformedSignature { sig, error } => due_to!(""),
                        //     VerificationError::MissingKey { sig } => {
                        //         due_to!("Mirror {} should be signed by {}.gpg, but there is no matching key hash {:?}",
                        //         self.mirror,
                        //         self.mirror,
                        //         sig.issuer_fingerprints().map(|x| x.to_string()).collect::<Vec<_>>());
                        //     }
                        //     _ => {}
                        //     // VerificationError::UnboundKey { sig, cert, error } => due_to!(""),
                        //     // VerificationError::BadKey { sig, ka, error } => due_to!(""),
                        //     // VerificationError::BadSignature { sig, ka, error } => due_to!(""),
                        // };
                        bail!("InRelease contains bad signature: {} .", e);
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
pub fn verify(s: &str, trust_files: Option<&str>, mirror: &str) -> Result<String> {
    let dir = std::fs::read_dir("/etc/apt/trusted.gpg.d")?;
    let mut cert_files = vec![];

    if let Some(trust_files) = trust_files {
        let trust_files = trust_files.split(',');
        for file in trust_files {
            let p = Path::new(file);
            if p.is_absolute() {
                cert_files.push(p.to_path_buf());
            } else {
                cert_files.push(Path::new("/etc/apt/trusted.gpg.d").join(file))
            }
        }
    } else {
        for i in dir.flatten() {
            let path = i.path();
            let ext = path.extension().and_then(|x| x.to_str());
            if ext == Some("gpg") || ext == Some("asc") {
                cert_files.push(i.path().to_path_buf());
            }
        }

        let trust_main = Path::new("/etc/apt/trusted.gpg").to_path_buf();

        if trust_main.is_file() {
            cert_files.push(trust_main);
        }
    }

    let p = StandardPolicy::new();
    let mut v = VerifierBuilder::from_bytes(s.as_bytes())?.with_policy(
        &p,
        None,
        InReleaseVerifier::new(&cert_files, mirror)?,
    )?;

    let mut res = String::new();
    v.read_to_string(&mut res)?;

    Ok(res)
}
