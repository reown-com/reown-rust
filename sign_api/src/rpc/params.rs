pub(super) mod session_delete;
pub(super) mod session_event;
pub(super) mod session_extend;
pub(super) mod session_ping;
pub(super) mod session_propose;
pub(super) mod session_request;
pub(super) mod session_settle;
pub(super) mod session_update;
pub(super) mod shared_types;

pub use {
    session_delete::*, session_event::*, session_extend::*, session_ping::*, session_propose::*,
    session_request::*, session_settle::*, session_update::*, shared_types::*,
};

use {
    paste::paste,
    serde::{Deserialize, Serialize},
    serde_json::Value,
    std::result::Result,
};

/// Errors covering Sign API payload parameter conversion issues.
#[derive(Debug, thiserror::Error)]
pub enum ParamsError {
    /// Sign API serialization/deserialization issues.
    #[error("Failure serializing/deserializing Sign API parameters: {0}")]
    Serde(#[from] serde_json::Error),
    /// Sign API invalid response tag.
    #[error("Response tag={0} does not match any of the Sign API methods")]
    ResponseTag(u32),
}

/// Relay protocol metadata.
///
///  https://specs.walletconnect.com/2.0/specs/clients/sign/rpc-methods
pub trait RelayProtocolMetadata {
    /// Retrieves IRN relay protocol metadata.
    ///
    /// Every method must return corresponding IRN metadata.
    fn irn_metadata(&self) -> IrnMetadata;
}

pub trait RelayProtocolHelpers {
    type Params;

    /// Converts "unnamed" payload parameters into typed.
    ///
    /// Example: success and error response payload does not specify the
    /// method. Thus the only way to deserialize the data into typed
    /// parameters, is to use the tag to determine the response method.
    ///
    /// This is a convenience method, so that users don't have to deal
    /// with the tags directly.
    fn irn_try_from_tag(value: Value, tag: u32) -> Result<Self::Params, ParamsError>;
}

/// Relay IRN protocol metadata.
///
/// https://specs.walletconnect.com/2.0/specs/servers/relay/relay-server-rpc
/// #definitions
#[derive(Debug, Clone, Copy)]
pub struct IrnMetadata {
    pub tag: u32,
    pub ttl: u64,
    pub prompt: bool,
}

// Convenience macro to de-duplicate implementation for different parameter sets.
macro_rules! impl_relay_protocol_metadata {
    ($param_type:ty,$meta:ident) => {
        paste! {
            impl RelayProtocolMetadata for $param_type {
                fn irn_metadata(&self) -> IrnMetadata {
                    match self {
                        [<$param_type>]::SessionPropose(_) => session_propose::[<IRN_ $meta:upper _METADATA>],
                        [<$param_type>]::SessionSettle(_) => session_settle::[<IRN_ $meta:upper _METADATA>],
                        [<$param_type>]::SessionUpdate(_) => session_update::[<IRN_ $meta:upper _METADATA>],
                        [<$param_type>]::SessionExtend(_) => session_extend::[<IRN_ $meta:upper _METADATA>],
                        [<$param_type>]::SessionRequest(_) => session_request::[<IRN_ $meta:upper _METADATA>],
                        [<$param_type>]::SessionEvent(_) => session_event::[<IRN_ $meta:upper _METADATA>],
                        [<$param_type>]::SessionDelete(_) => session_delete::[<IRN_ $meta:upper _METADATA>],
                        [<$param_type>]::SessionPing(_) => session_ping::[<IRN_ $meta:upper _METADATA>],
                    }
                }
            }
        }
    }
}

// Convenience macro to de-duplicate implementation for different parameter sets.
macro_rules! impl_relay_protocol_helpers {
    ($param_type:ty) => {
        paste! {
            impl RelayProtocolHelpers for $param_type {
                type Params = Self;

                fn irn_try_from_tag(value: Value, tag: u32) -> Result<Self::Params, ParamsError> {
                    if tag == session_propose::IRN_RESPONSE_METADATA.tag {
                        Ok(Self::SessionPropose(serde_json::from_value(value)?))
                    } else if tag == session_settle::IRN_RESPONSE_METADATA.tag {
                        Ok(Self::SessionSettle(serde_json::from_value(value)?))
                    } else if tag == session_update::IRN_RESPONSE_METADATA.tag {
                        Ok(Self::SessionUpdate(serde_json::from_value(value)?))
                    } else if tag == session_extend::IRN_RESPONSE_METADATA.tag {
                        Ok(Self::SessionExtend(serde_json::from_value(value)?))
                    } else if tag == session_request::IRN_RESPONSE_METADATA.tag {
                        Ok(Self::SessionRequest(serde_json::from_value(value)?))
                    } else if tag == session_event::IRN_RESPONSE_METADATA.tag {
                        Ok(Self::SessionEvent(serde_json::from_value(value)?))
                    } else if tag == session_delete::IRN_RESPONSE_METADATA.tag {
                        Ok(Self::SessionDelete(serde_json::from_value(value)?))
                    } else if tag == session_ping::IRN_RESPONSE_METADATA.tag {
                        Ok(Self::SessionPing(serde_json::from_value(value)?))
                    } else {
                        Err(ParamsError::ResponseTag(tag))
                    }
                }
            }
        }
    };
}

/// Sign API request parameters.
///
/// https://specs.walletconnect.com/2.0/specs/clients/sign/rpc-methods
/// https://specs.walletconnect.com/2.0/specs/clients/sign/data-structures
#[derive(Debug, Serialize, Eq, Deserialize, Clone, PartialEq)]
#[serde(tag = "method", content = "params")]
pub enum RequestParams {
    #[serde(rename = "wc_sessionPropose")]
    SessionPropose(SessionProposeRequest),
    #[serde(rename = "wc_sessionSettle")]
    SessionSettle(SessionSettleRequest),
    #[serde(rename = "wc_sessionUpdate")]
    SessionUpdate(SessionUpdateRequest),
    #[serde(rename = "wc_sessionExtend")]
    SessionExtend(SessionExtendRequest),
    #[serde(rename = "wc_sessionRequest")]
    SessionRequest(SessionRequestRequest),
    #[serde(rename = "wc_sessionEvent")]
    SessionEvent(SessionEventRequest),
    #[serde(rename = "wc_sessionDelete")]
    SessionDelete(SessionDeleteRequest),
    #[serde(rename = "wc_sessionPing")]
    SessionPing(()),
}
impl_relay_protocol_metadata!(RequestParams, request);

/// https://www.jsonrpc.org/specification#response_object
///
/// JSON RPC 2.0 response object can either carry success or error data.
/// Please note, that relay protocol metadata is used to disambiguate the
/// response data.
///
/// For example:
/// `RelayProtocolHelpers::irn_try_from_tag` is used to deserialize an opaque
/// response data into the typed parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseParams {
    /// A response with a result.
    #[serde(rename = "result")]
    Success(Value),

    /// A response for a failed request.
    #[serde(rename = "error")]
    Err(Value),
}

/// Typed success response parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseParamsSuccess {
    SessionPropose(SessionProposeResponse),
    SessionSettle(bool),
    SessionUpdate(bool),
    SessionExtend(bool),
    SessionRequest(bool),
    SessionEvent(bool),
    SessionDelete(bool),
    SessionPing(bool),
}
impl_relay_protocol_metadata!(ResponseParamsSuccess, response);
impl_relay_protocol_helpers!(ResponseParamsSuccess);

impl TryFrom<ResponseParamsSuccess> for ResponseParams {
    type Error = ParamsError;

    fn try_from(value: ResponseParamsSuccess) -> Result<Self, Self::Error> {
        Ok(Self::Success(serde_json::to_value(value)?))
    }
}

/// Response error data.
///
/// The documentation states that both fields are required.
/// However, on session expiry error, "empty" error is received.
#[derive(Debug, Clone, Eq, Serialize, Deserialize, PartialEq)]
pub struct ErrorParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub code: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub message: Option<String>,
}

/// Typed error response parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseParamsError {
    SessionPropose(ErrorParams),
    SessionSettle(ErrorParams),
    SessionUpdate(ErrorParams),
    SessionExtend(ErrorParams),
    SessionRequest(ErrorParams),
    SessionEvent(ErrorParams),
    SessionDelete(ErrorParams),
    SessionPing(ErrorParams),
}
impl_relay_protocol_metadata!(ResponseParamsError, response);
impl_relay_protocol_helpers!(ResponseParamsError);

impl TryFrom<ResponseParamsError> for ResponseParams {
    type Error = ParamsError;

    fn try_from(value: ResponseParamsError) -> Result<Self, Self::Error> {
        Ok(Self::Err(serde_json::to_value(value)?))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use anyhow::Result;
    use serde::de::DeserializeOwned;
    use serde_json;

    /// Trims json of the whitespaces and newlines.
    ///
    /// Allows to use "pretty json" in unittest, and still get consistent
    /// results post serialization/deserialization.
    pub fn param_json_trim(json: &str) -> String {
        json.chars()
            .filter(|c| !c.is_whitespace() && *c != '\n')
            .collect::<String>()
    }

    /// Tests input json serialization/deserialization into the specified type.
    pub fn param_serde_test<T>(json: &str) -> Result<()>
    where
        T: Serialize + DeserializeOwned,
    {
        let expected = param_json_trim(json);
        let deserialized: T = serde_json::from_str(&expected)?;
        let actual = serde_json::to_string(&deserialized)?;

        assert_eq!(expected, actual);

        Ok(())
    }
}
