//! The crate exports common types used when interacting with messages between
//! clients. This also includes communication over HTTP between relays.

mod params;

use {
    chrono::Utc,
    serde::{Deserialize, Serialize},
    std::{fmt::Debug, sync::Arc},
};

pub use params::*;

/// Version of the WalletConnect protocol that we're implementing.
pub const JSON_RPC_VERSION_STR: &str = "2.0";

pub static JSON_RPC_VERSION: once_cell::sync::Lazy<Arc<str>> =
    once_cell::sync::Lazy::new(|| Arc::from(JSON_RPC_VERSION_STR));

/// Errors covering payload validation problems.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid request ID")]
    RequestId,

    #[error("Invalid JSON RPC version")]
    JsonRpcVersion,
}

/// Errors caught while processing the Sign API request/response. These should
/// be specific enough for the clients to make sense of the problem.
#[derive(Debug, thiserror::Error)]
pub enum GenericError {
    /// Request parameters validation failed.
    #[error("Request validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Request/response serialization error.
    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Enum representing a JSON RPC payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Payload {
    Request(Request),
    Response(Response),
}

impl From<Request> for Payload {
    fn from(value: Request) -> Self {
        Payload::Request(value)
    }
}

impl From<Response> for Payload {
    fn from(value: Response) -> Self {
        Payload::Response(value)
    }
}

impl Payload {
    pub fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::Request(request) => request.validate(),
            Self::Response(response) => response.validate(),
        }
    }

    pub fn irn_tag_in_range(tag: u32) -> bool {
        (1100..=1115).contains(&tag)
    }
}

/// Data structure representing a JSON RPC request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// ID this message corresponds to.
    pub id: u64,

    /// The JSON RPC version.
    pub jsonrpc: Arc<str>,

    /// The parameters required to fulfill this request.
    #[serde(flatten)]
    pub params: RequestParams,
}

impl Request {
    /// Create a new instance.
    pub fn new(params: RequestParams) -> Self {
        Self {
            id: Utc::now().timestamp_micros() as u64,
            jsonrpc: JSON_RPC_VERSION_STR.into(),
            params,
        }
    }

    /// Validates the request payload.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.jsonrpc.as_ref() != JSON_RPC_VERSION_STR {
            return Err(ValidationError::JsonRpcVersion);
        }

        Ok(())
    }
}

/// Data structure representing JSON RPC response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    /// ID this message corresponds to.
    pub id: u64,

    /// RPC version.
    pub jsonrpc: Arc<str>,

    /// The parameters required to fulfill this response.
    #[serde(flatten)]
    pub params: ResponseParams,
}

impl Response {
    /// Create a new instance.
    pub fn new(id: u64, params: ResponseParams) -> Self {
        Self {
            id,
            jsonrpc: JSON_RPC_VERSION.clone(),
            params,
        }
    }

    /// Validates the parameters.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.jsonrpc.as_ref() != JSON_RPC_VERSION_STR {
            return Err(ValidationError::JsonRpcVersion);
        }

        Ok(())
    }
}
