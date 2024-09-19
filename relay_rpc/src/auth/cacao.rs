use {
    self::{
        header::Header,
        payload::Payload,
        signature::{get_rpc_url::GetRpcUrl, Signature},
    },
    alloy::primitives::hex::FromHexError,
    core::fmt::Debug,
    serde::{Deserialize, Serialize},
    serde_json::value::RawValue,
    std::{
        fmt::{Display, Write},
        sync::Arc,
    },
};

pub mod header;
pub mod payload;
pub mod signature;

/// Errors that can occur during Cacao verification.
#[derive(Debug, thiserror::Error)]
pub enum CacaoError {
    #[error("Header `t` value unsupported: {0}")]
    HeaderTypeUnsupported(Arc<str>),

    #[error("Invalid or missing identity key in payload resources")]
    PayloadIdentityKey,

    #[error("Invalid payload resources")]
    PayloadResources,

    #[error("Invalid address")]
    AddressInvalid,

    #[error("Address not EIP-191")]
    AddressNotEip191(FromHexError),

    #[error("EIP-1271 signatures not supported")]
    Eip1271NotSupported,

    #[error("EIP-6492 signatures not supported")]
    Eip6492NotSupported,

    #[error("Unsupported signature type")]
    UnsupportedSignature,

    #[error("Provider not available for that chain")]
    ProviderNotAvailable,

    #[error("Unable to verify")]
    Verification,

    #[error("Internal EIP-1271 resolution error: {0}")]
    Eip1271Internal(
        alloy::rpc::json_rpc::RpcError<alloy::transports::TransportErrorKind, Box<RawValue>>,
    ),

    #[error("Internal EIP-6492 resolution error: {0}")]
    Eip6492Internal(
        alloy::rpc::json_rpc::RpcError<alloy::transports::TransportErrorKind, Box<RawValue>>,
    ),
}

impl From<std::fmt::Error> for CacaoError {
    fn from(_: std::fmt::Error) -> Self {
        CacaoError::PayloadResources
    }
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

    pub async fn verify(&self, provider: Option<&impl GetRpcUrl>) -> Result<(), CacaoError> {
        self.p.validate()?;
        self.h.validate()?;
        self.s.verify(self, provider).await
    }

    pub fn siwe_message(&self) -> Result<String, CacaoError> {
        self.caip122_message(Self::ETHEREUM)
    }

    pub fn caip122_message(&self, chain_name: &str) -> Result<String, CacaoError> {
        let mut message = format!(
            "{} wants you to sign in with your {} account:\n{}\n\n",
            self.p.domain,
            chain_name,
            self.p.address()?
        );

        if let Some(statement) = &self.p.statement {
            writeln!(message, "{}", statement)?;
        }

        write!(
            message,
            "\nURI: {}\nVersion: {}\nChain ID: {}\nNonce: {}\nIssued At: {}",
            self.p.aud,
            self.p.version,
            self.p.chain_id()?,
            self.p.nonce,
            self.p.iat
        )?;

        if let Some(exp) = &self.p.exp {
            write!(message, "\nExpiration Time: {}", exp)?;
        }

        if let Some(nbf) = &self.p.nbf {
            write!(message, "\nNot Before: {}", nbf)?;
        }

        if let Some(request_id) = &self.p.request_id {
            write!(message, "\nRequest ID: {}", request_id)?;
        }

        if let Some(resources) = &self.p.resources {
            if !resources.is_empty() {
                write!(message, "\nResources:")?;

                for resource in resources {
                    write!(message, "\n- {}", resource)?;
                }
            }
        }

        Ok(message)
    }
}

#[cfg(test)]
mod tests;
