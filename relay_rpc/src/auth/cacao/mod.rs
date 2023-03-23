use {
    self::{header::Header, payload::Payload, signature::Signature},
    core::fmt::Debug,
    serde::{Deserialize, Serialize},
    std::fmt::{Display, Write as _},
    thiserror::Error as ThisError,
};

pub mod header;
pub mod payload;
pub mod signature;

/// Errors that can occur during JWT verification
#[derive(Debug, ThisError)]
pub enum CacaoError {
    #[error("Invalid header")]
    Header,

    #[error("Invalid or missing identity key in payload resources")]
    PayloadIdentityKey,

    #[error("Invalid payload resources")]
    PayloadResources,

    #[error("Unsupported signature type")]
    UnsupportedSignature,

    #[error("Unable to verify")]
    Verification,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Version {
    V1 = 1,
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let version = String::deserialize(deserializer)?;
        match version.as_str() {
            "1" => Ok(Version::V1),
            _ => Err(serde::de::Error::custom("Invalid version")),
        }
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}", *self as u8))
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub struct Cacao {
    pub h: Header,
    pub p: Payload,
    pub s: Signature,
}

impl Cacao {
    const ETHEREUM: &'static str = "Ethereum";

    pub fn verify(&self) -> Result<bool, CacaoError> {
        self.p.is_valid()?;
        self.h.is_valid()?;
        self.s.verify(self)
    }

    pub fn siwe_message(&self) -> Result<String, CacaoError> {
        self.caip122_message(Self::ETHEREUM)
    }

    pub fn caip122_message(&self, chain_name: &str) -> Result<String, CacaoError> {
        let mut message = format!(
            "{} wants you to sign in with your {} account:\n{}\n",
            self.p.domain,
            chain_name,
            self.p.address()?
        );

        if let Some(statement) = &self.p.statement {
            let _ = write!(message, "\n{}\n", statement);
        }

        let _ = write!(
            message,
            "\nURI: {}\nVersion: {}\nChain ID: {}\nNonce: {}\nIssued At: {}",
            self.p.aud,
            self.p.version,
            self.p.chain_id()?,
            self.p.nonce,
            self.p.iat
        );

        if let Some(exp) = &self.p.exp {
            let _ = write!(message, "\nExpiration Time: {}", exp);
        }

        if let Some(nbf) = &self.p.nbf {
            let _ = write!(message, "\nNot Before: {}", nbf);
        }

        if let Some(request_id) = &self.p.request_id {
            let _ = write!(message, "\nRequest ID: {}", request_id);
        }

        if let Some(resources) = &self.p.resources {
            if !resources.is_empty() {
                let _ = write!(message, "\nResources:");
                resources.iter().for_each(|resource| {
                    let _ = write!(message, "\n- {}", resource);
                });
            }
        }

        Ok(message)
    }
}

#[cfg(test)]
mod tests;
