pub use tokio_tungstenite::tungstenite::protocol::CloseFrame;

pub type WsError = tokio_tungstenite::tungstenite::Error;
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Wrapper around the websocket [`CloseFrame`] providing info about the
/// connection closing reason.
#[derive(Debug, Clone)]
pub struct CloseReason(pub Option<CloseFrame<'static>>);

impl std::fmt::Display for CloseReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(frame) = &self.0 {
            frame.fmt(f)
        } else {
            f.write_str("<close frame unavailable>")
        }
    }
}

/// Errors generated while parsing
/// [`ConnectionOptions`][crate::ConnectionOptions] and creating an HTTP request
/// for the websocket connection.
#[derive(Debug, thiserror::Error)]
pub enum RequestBuildError {
    #[error("Failed to serialize connection query: {0}")]
    Query(#[from] serde_qs::Error),

    #[error("Failed to add request headers")]
    Headers,

    #[error("Failed to create websocket request: {0}")]
    Other(WsError),
}

/// Possible Relay client errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to build connection request: {0}")]
    RequestBuilder(#[from] RequestBuildError),

    #[error("Failed to connect: {0}")]
    ConnectionFailed(WsError),

    #[error("Connection closed: {0}")]
    ConnectionClosed(CloseReason),

    #[error("Failed to close connection: {0}")]
    ClosingFailed(WsError),

    #[error("Not connected")]
    NotConnected,

    #[error("Websocket error: {0}")]
    Socket(WsError),

    #[error("Internal error: Channel closed")]
    ChannelClosed,

    #[error("Internal error: Duplicate request ID")]
    DuplicateRequestId,

    #[error("Invalid response ID")]
    InvalidResponseId,

    #[error("Serialization failed: {0}")]
    Serialization(serde_json::Error),

    #[error("Deserialization failed: {0}")]
    Deserialization(serde_json::Error),

    #[error("RPC error ({code}): {message}")]
    Rpc { code: i32, message: String },

    #[error("Invalid request type")]
    InvalidRequestType,
}
