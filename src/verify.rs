use std::{io::Read, path::Path};

use anyhow::{bail, Result};
use sequoia_openpgp::{
    parse::{
        stream::{MessageLayer, MessageStructure, VerificationHelper, VerifierBuilder},
        Parse,
    },
    policy::StandardPolicy,
    Cert, KeyHandle,
};

pub struct InReleaseVerifier {
    certs: Vec<Cert>,
}

impl InReleaseVerifier {
    pub fn new<P: AsRef<Path>>(cert_paths: &[P]) -> Result<Self> {
        let mut certs: Vec<Cert> = Vec::new();
        for path in cert_paths.iter() {
            certs.push(Cert::from_file(path)?);
        }

        Ok(InReleaseVerifier { certs })
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
pub fn verify(s: &str, trust_files: Option<&str>) -> Result<String> {
    let dir = std::fs::read_dir("/etc/apt/trusted.gpg.d")?;
    let mut cert_files = vec![];

    if trust_files.is_none() {
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
    } else {
        let trust_files = trust_files.unwrap().split(",");
        for file in trust_files {
            let p = Path::new(file);
            if p.is_absolute() {
                cert_files.push(p.to_path_buf());
            } else {
                cert_files.push(Path::new("/etc/apt/trusted.gpg.d").join(file))
            }
        }
    }

    let p = StandardPolicy::new();
    let mut v = VerifierBuilder::from_bytes(s.as_bytes())?.with_policy(
        &p,
        None,
        InReleaseVerifier::new(&cert_files)?,
    )?;

    let mut res = String::new();
    v.read_to_string(&mut res)?;

    Ok(res)
}
