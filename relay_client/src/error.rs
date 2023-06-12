pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Errors generated while parsing
/// [`ConnectionOptions`][crate::ConnectionOptions] and creating an HTTP request
/// for the websocket connection.
#[derive(Debug, thiserror::Error)]
pub enum RequestBuildError {
    #[error("Failed to serialize connection query: {0}")]
    Query(#[from] serde_qs::Error),

    #[error("Failed to add request headers")]
    Headers,

    #[error("Failed to parse connection URL: {0}")]
    Url(#[from] url::ParseError),

    #[error("Failed to create websocket request: {0}")]
    WebsocketClient(#[from] crate::websocket::WebsocketClientError),

    #[error("Failed to create HTTP request: {0}")]
    HttpClient(#[from] crate::http::HttpClientError),
}

/// Possible Relay client errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to build connection request: {0}")]
    RequestBuilder(#[from] RequestBuildError),

    #[error("Websocket client error: {0}")]
    WebsocketClient(#[from] crate::websocket::WebsocketClientError),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] crate::http::HttpClientError),

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
