use {
    super::ErrorData,
    std::fmt::{Debug, Display},
};

/// Provides serialization to and from string tags. This has a blanket
/// implementation for all error types that derive [`strum::EnumString`] and
/// [`strum::IntoStaticStr`].
pub trait ServiceError: Sized + Debug + Display + PartialEq + Send + 'static {
    fn from_tag(tag: &str) -> Result<Self, InvalidErrorData>;

    fn tag(&self) -> &'static str;
}

impl<T> ServiceError for T
where
    T: for<'a> TryFrom<&'a str> + Debug + Display + PartialEq + Send + 'static,
    for<'a> &'static str: From<&'a T>,
{
    fn from_tag(tag: &str) -> Result<Self, InvalidErrorData> {
        tag.try_into().map_err(|_| InvalidErrorData)
    }

    fn tag(&self) -> &'static str {
        self.into()
    }
}

#[derive(Debug, thiserror::Error, strum::EnumString, strum::IntoStaticStr, PartialEq, Eq)]
pub enum AuthError {
    #[error("Project not found")]
    ProjectNotFound,

    #[error("Project ID not specified")]
    ProjectIdNotSpecified,

    #[error("Project inactive")]
    ProjectInactive,

    #[error("Origin not allowed")]
    OriginNotAllowed,

    #[error("Invalid JWT")]
    InvalidJwt,

    #[error("Missing JWT")]
    MissingJwt,

    #[error("Country blocked")]
    CountryBlocked,
}

/// Request payload validation problems.
#[derive(
    Debug, Clone, thiserror::Error, strum::EnumString, strum::IntoStaticStr, PartialEq, Eq,
)]
pub enum PayloadError {
    #[error("Invalid request method")]
    InvalidMethod,

    #[error("Invalid request parameters")]
    InvalidParams,

    #[error("Payload size exceeded")]
    PayloadSizeExceeded,

    #[error("Topic decoding failed")]
    InvalidTopic,

    #[error("Subscription ID decoding failed")]
    InvalidSubscriptionId,

    #[error("Invalid request ID")]
    InvalidRequestId,

    #[error("Invalid JSON RPC version")]
    InvalidJsonRpcVersion,

    #[error("The batch contains too many items")]
    BatchLimitExceeded,

    #[error("The batch contains no items")]
    BatchEmpty,

    #[error("Failed to deserialize request")]
    Serialization,
}

#[derive(Debug, thiserror::Error, strum::EnumString, strum::IntoStaticStr, PartialEq, Eq)]
pub enum InternalError {
    #[error("Storage operation failed")]
    StorageError,

    #[error("Failed to serialize response")]
    Serialization,

    #[error("Internal error")]
    Unknown,
}

/// Errors caught while processing the request. These are meant to be serialized
/// into [`super::ErrorResponse`], and should be specific enough for the clients
/// to make sense of the problem.
#[derive(Debug, thiserror::Error, strum::IntoStaticStr, PartialEq, Eq)]
pub enum Error<T> {
    #[error("Auth error: {0}")]
    Auth(#[from] AuthError),

    #[error("Invalid payload: {0}")]
    Payload(#[from] PayloadError),

    #[error("Request handler error: {0}")]
    Handler(T),

    #[error("Internal error: {0}")]
    Internal(#[from] InternalError),

    #[error("Too many requests")]
    TooManyRequests,
}

impl<T: ServiceError> Error<T> {
    pub fn code(&self) -> i32 {
        match self {
            Self::Auth(_) => CODE_AUTH,
            Self::TooManyRequests => CODE_TOO_MANY_REQUESTS,
            Self::Payload(_) => CODE_PAYLOAD,
            Self::Handler(_) => CODE_HANDLER,
            Self::Internal(_) => CODE_INTERNAL,
        }
    }

    pub fn tag(&self) -> &'static str {
        match &self {
            Self::Auth(err) => err.tag(),
            Self::Payload(err) => err.tag(),
            Self::Handler(err) => err.tag(),
            Self::Internal(err) => err.tag(),
            Self::TooManyRequests => self.into(),
        }
    }
}

pub const CODE_AUTH: i32 = 3000;
pub const CODE_TOO_MANY_REQUESTS: i32 = 3001;
pub const CODE_PAYLOAD: i32 = -32600;
pub const CODE_HANDLER: i32 = -32000;
pub const CODE_INTERNAL: i32 = -32603;

#[derive(Debug, thiserror::Error)]
#[error("Invalid error data")]
pub struct InvalidErrorData;

impl<T: ServiceError> TryFrom<ErrorData> for Error<T> {
    type Error = InvalidErrorData;

    fn try_from(err: ErrorData) -> Result<Self, Self::Error> {
        let tag = &err.data;

        let err = match err.code {
            CODE_AUTH => Error::Auth(try_parse_error(tag)?),
            CODE_TOO_MANY_REQUESTS => Error::TooManyRequests,
            CODE_PAYLOAD => Error::Payload(try_parse_error(tag)?),
            CODE_HANDLER => Error::Handler(try_parse_error(tag)?),
            CODE_INTERNAL => Error::Internal(try_parse_error(tag)?),
            _ => return Err(InvalidErrorData),
        };

        Ok(err)
    }
}

#[inline]
fn try_parse_error<T: ServiceError>(tag: &Option<String>) -> Result<T, InvalidErrorData> {
    tag.as_deref().ok_or(InvalidErrorData).map(T::from_tag)?
}

impl<T: ServiceError> From<Error<T>> for ErrorData {
    fn from(err: Error<T>) -> Self {
        Self {
            code: err.code(),
            message: err.to_string(),
            data: Some(err.tag().to_owned()),
        }
    }
}
