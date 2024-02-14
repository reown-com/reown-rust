use relay_rpc::rpc::{self, error::ServiceError};

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
pub enum ClientError {
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

    #[error("Invalid error response")]
    InvalidErrorResponse,

    #[error("Serialization failed: {0}")]
    Serialization(serde_json::Error),

    #[error("Deserialization failed: {0}")]
    Deserialization(serde_json::Error),

    #[error("RPC error: code={code} data={data:?} message={message}")]
    Rpc {
        code: i32,
        message: String,
        data: Option<String>,
    },

    #[error("Invalid request type")]
    InvalidRequestType,
}

impl From<rpc::ErrorData> for ClientError {
    fn from(err: rpc::ErrorData) -> Self {
        Self::Rpc {
            code: err.code,
            message: err.message,
            data: err.data,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error<T: ServiceError> {
    /// Client errors encountered while performing the request.
    #[error(transparent)]
    Client(ClientError),

    /// Error response received from the relay.
    #[error(transparent)]
    Response(#[from] rpc::Error<T>),
}

impl<T: ServiceError> From<ClientError> for Error<T> {
    fn from(err: ClientError) -> Self {
        match err {
            ClientError::Rpc {
                code,
                message,
                data,
            } => {
                let err = rpc::ErrorData {
                    code,
                    message,
                    data,
                };

                match rpc::Error::try_from(err) {
                    Ok(err) => Error::Response(err),

                    Err(_) => Error::Client(ClientError::InvalidErrorResponse),
                }
            }

            _ => Error::Client(err),
        }
    }
}
