use std::fmt::Write;
use std::str::FromStr;

use anyhow::{Error, Result, anyhow, bail};
use base64::prelude::{BASE64_STANDARD, Engine as _};

#[derive(Debug, Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct SriHash {
    pub algorithm: Algorithm,
    pub digest: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum Algorithm {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

impl FromStr for SriHash {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parsed = s.trim().split('-');

        let algorithm: Algorithm = parsed
            .next()
            .and_then(|s| FromStr::from_str(s).ok())
            .ok_or(anyhow!("Failed to parse hash algorithm"))?;

        let digest = parsed
            .next()
            .and_then(|s| BASE64_STANDARD.decode(s).ok())
            .ok_or(anyhow!("Failed to decode hash digest"))?;

        Ok(Self { algorithm, digest })
    }
}

impl FromStr for Algorithm {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let matched = match s {
            "md5" => Self::Md5,
            "sha1" => Self::Sha1,
            "sha256" => Self::Sha256,
            "sha512" => Self::Sha512,
            _ => bail!("Failed to parse hash algorithm"),
        };
        Ok(matched)
    }
}

impl SriHash {
    /// Return the digest as a lower hex encoded string.
    pub fn hex_digest(&self) -> String {
        let mut buffer = String::new();
        for byte in &self.digest {
            let _ = write!(&mut buffer, "{byte:02x}");
        }
        buffer
    }
}
