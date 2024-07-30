//! The crate exports common types used when interacting with messages between
//! clients. This also includes communication over HTTP between relays.

use {
    crate::domain::{DidKey, MessageId, SubscriptionId, Topic},
    serde::{de::DeserializeOwned, Deserialize, Serialize},
    std::{fmt::Debug, sync::Arc},
};
pub use {error::*, watch::*};

pub mod error;
pub mod msg_id;
#[cfg(test)]
mod tests;
pub mod watch;

/// Version of the WalletConnect protocol that we're implementing.
pub const JSON_RPC_VERSION_STR: &str = "2.0";

pub static JSON_RPC_VERSION: once_cell::sync::Lazy<Arc<str>> =
    once_cell::sync::Lazy::new(|| Arc::from(JSON_RPC_VERSION_STR));

/// The maximum number of topics allowed for a batch subscribe request.
///
/// See <https://github.com/WalletConnect/walletconnect-docs/blob/main/docs/specs/servers/relay/relay-server-rpc.md>
pub const MAX_SUBSCRIPTION_BATCH_SIZE: usize = 500;

/// The maximum number of topics allowed for a batch fetch request.
///
/// See <https://github.com/WalletConnect/walletconnect-docs/blob/main/docs/specs/servers/relay/relay-server-rpc.md>
pub const MAX_FETCH_BATCH_SIZE: usize = 500;

/// The maximum number of receipts allowed for a batch receive request.
///
/// See <https://github.com/WalletConnect/walletconnect-docs/blob/main/docs/specs/servers/relay/relay-server-rpc.md>
pub const MAX_RECEIVE_BATCH_SIZE: usize = 500;

pub trait Serializable:
    Debug + Clone + PartialEq + Eq + Serialize + DeserializeOwned + Send + Sync + 'static
{
}
impl<T> Serializable for T where
    T: Debug + Clone + PartialEq + Eq + Serialize + DeserializeOwned + Send + Sync + 'static
{
}

/// Trait that adds validation capabilities and strong typing to errors and
/// successful responses. Implemented for all possible RPC request types.
pub trait ServiceRequest: Serializable {
    /// The error representing a failed request.
    type Error: ServiceError;

    /// The type of a successful response.
    type Response: Serializable;

    /// Validates the request parameters.
    fn validate(&self) -> Result<(), PayloadError> {
        Ok(())
    }

    fn into_params(self) -> Params;
}

/// Enum representing a JSON RPC payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Payload {
    /// An inbound request.
    Request(Request),

    /// An outbound response.
    Response(Response),
}

impl Payload {
    /// Returns the message ID contained within the payload.
    pub fn id(&self) -> MessageId {
        match self {
            Self::Request(req) => req.id,
            Self::Response(Response::Success(r)) => r.id,
            Self::Response(Response::Error(r)) => r.id,
        }
    }

    pub fn validate(&self) -> Result<(), PayloadError> {
        match self {
            Self::Request(request) => request.validate(),
            Self::Response(response) => response.validate(),
        }
    }
}

impl<T> From<T> for Payload
where
    T: Into<ErrorResponse>,
{
    fn from(value: T) -> Self {
        Self::Response(Response::Error(value.into()))
    }
}

/// Enum representing a JSON RPC response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    /// A response with a result.
    Success(SuccessfulResponse),

    /// A response for a failed request.
    Error(ErrorResponse),
}

impl Response {
    pub fn id(&self) -> MessageId {
        match self {
            Self::Success(response) => response.id,
            Self::Error(response) => response.id,
        }
    }

    /// Validates the response parameters.
    pub fn validate(&self) -> Result<(), PayloadError> {
        match self {
            Self::Success(response) => response.validate(),
            Self::Error(response) => response.validate(),
        }
    }
}

/// Data structure representing a successful JSON RPC response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuccessfulResponse {
    /// ID this message corresponds to.
    pub id: MessageId,

    /// RPC version.
    pub jsonrpc: Arc<str>,

    /// The result for the message.
    pub result: serde_json::Value,
}

impl SuccessfulResponse {
    /// Create a new instance.
    pub fn new(id: MessageId, result: serde_json::Value) -> Self {
        Self {
            id,
            jsonrpc: JSON_RPC_VERSION.clone(),
            result,
        }
    }

    /// Validates the parameters.
    pub fn validate(&self) -> Result<(), PayloadError> {
        if self.jsonrpc.as_ref() != JSON_RPC_VERSION_STR {
            Err(PayloadError::InvalidJsonRpcVersion)
        } else {
            // We can't really validate `serde_json::Value` without knowing the expected
            // value type.
            Ok(())
        }
    }
}

/// Data structure representing a JSON RPC error response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// ID this message corresponds to.
    pub id: MessageId,

    /// RPC version.
    pub jsonrpc: Arc<str>,

    /// The ErrorResponse corresponding to this message.
    pub error: ErrorData,
}

impl ErrorResponse {
    /// Create a new instance.
    pub fn new(id: MessageId, error: impl Into<ErrorData>) -> Self {
        Self {
            id,
            jsonrpc: JSON_RPC_VERSION.clone(),
            error: error.into(),
        }
    }

    /// Validates the parameters.
    pub fn validate(&self) -> Result<(), PayloadError> {
        if self.jsonrpc.as_ref() != JSON_RPC_VERSION_STR {
            Err(PayloadError::InvalidJsonRpcVersion)
        } else {
            Ok(())
        }
    }
}

/// Data structure representing error response params.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorData {
    /// Error code.
    pub code: i32,

    /// Error message.
    pub message: String,

    /// Error data, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

#[derive(Debug, thiserror::Error, strum::EnumString, strum::IntoStaticStr, PartialEq, Eq)]
pub enum SubscriptionError {
    #[error("Subscriber limit exceeded")]
    SubscriberLimitExceeded,
}

/// Subscription request parameters. This request does not require the
/// subscription to be fully processed, and returns as soon as the server
/// receives it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Subscribe {
    /// The topic to subscribe to.
    pub topic: Topic,
}

impl ServiceRequest for Subscribe {
    type Error = SubscriptionError;
    type Response = SubscriptionId;

    fn validate(&self) -> Result<(), PayloadError> {
        self.topic
            .decode()
            .map_err(|_| PayloadError::InvalidTopic)?;

        Ok(())
    }

    fn into_params(self) -> Params {
        Params::Subscribe(self)
    }
}

/// Subscription request parameters. This request awaits the subscription to be
/// fully processed and returns possible errors.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscribeBlocking {
    /// The topic to subscribe to.
    pub topic: Topic,
}

impl ServiceRequest for SubscribeBlocking {
    type Error = SubscriptionError;
    type Response = SubscriptionId;

    fn validate(&self) -> Result<(), PayloadError> {
        self.topic
            .decode()
            .map_err(|_| PayloadError::InvalidTopic)?;

        Ok(())
    }

    fn into_params(self) -> Params {
        Params::SubscribeBlocking(self)
    }
}

/// Data structure representing unsubscribe request params.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Unsubscribe {
    /// The topic to unsubscribe from.
    pub topic: Topic,
}

impl ServiceRequest for Unsubscribe {
    type Error = SubscriptionError;
    type Response = bool;

    fn validate(&self) -> Result<(), PayloadError> {
        self.topic
            .decode()
            .map_err(|_| PayloadError::InvalidTopic)?;

        // FIXME: Subscription ID validation is currently disabled, since SDKs do not
        // use the actual IDs generated by the relay, and instead send some randomized
        // values. We should either fix SDKs to ensure they properly utilize the IDs, or
        // just remove it from the payload.

        // self.subscription_id
        //     .decode()
        //     .map_err(ValidationError::SubscriptionIdDecoding)?;

        Ok(())
    }

    fn into_params(self) -> Params {
        Params::Unsubscribe(self)
    }
}

/// Data structure representing fetch request params.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FetchMessages {
    /// The topic of the messages to fetch.
    pub topic: Topic,
}

impl ServiceRequest for FetchMessages {
    type Error = GenericError;
    type Response = FetchResponse;

    fn validate(&self) -> Result<(), PayloadError> {
        self.topic
            .decode()
            .map_err(|_| PayloadError::InvalidTopic)?;

        Ok(())
    }

    fn into_params(self) -> Params {
        Params::FetchMessages(self)
    }
}

/// Data structure representing fetch response.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchResponse {
    /// Array of messages fetched from the mailbox.
    pub messages: Vec<SubscriptionData>,

    /// Flag that indicates whether the client should keep fetching the
    /// messages.
    pub has_more: bool,
}

/// Multi-topic subscription request parameters. This request does not require
/// all subscriptions to be fully processed, and returns as soon as the server
/// receives it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BatchSubscribe {
    /// The topics to subscribe to.
    pub topics: Vec<Topic>,
}

impl BatchSubscribe {
    fn validate_topics(topics: &[Topic]) -> Result<(), PayloadError> {
        let batch_size = topics.len();

        if batch_size == 0 {
            return Err(PayloadError::BatchEmpty);
        }

        if batch_size > MAX_SUBSCRIPTION_BATCH_SIZE {
            return Err(PayloadError::BatchLimitExceeded);
        }

        for topic in topics {
            topic.decode().map_err(|_| PayloadError::InvalidTopic)?;
        }

        Ok(())
    }
}

impl ServiceRequest for BatchSubscribe {
    type Error = SubscriptionError;
    type Response = Vec<SubscriptionId>;

    fn validate(&self) -> Result<(), PayloadError> {
        Self::validate_topics(&self.topics)
    }

    fn into_params(self) -> Params {
        Params::BatchSubscribe(self)
    }
}

/// Multi-topic subscription request parameters. This request awaits all
/// subscriptions to be fully processed and returns possible errors per topic.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BatchSubscribeBlocking {
    /// The topics to subscribe to.
    pub topics: Vec<Topic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SubscriptionResult {
    Id(SubscriptionId),
    Error(ErrorData),
}

impl ServiceRequest for BatchSubscribeBlocking {
    type Error = SubscriptionError;
    type Response = Vec<SubscriptionResult>;

    fn validate(&self) -> Result<(), PayloadError> {
        BatchSubscribe::validate_topics(&self.topics)
    }

    fn into_params(self) -> Params {
        Params::BatchSubscribeBlocking(self)
    }
}

/// Multi-topic unsubscription request parameters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BatchUnsubscribe {
    /// The subscriptions to unsubscribe from.
    pub subscriptions: Vec<Unsubscribe>,
}

impl ServiceRequest for BatchUnsubscribe {
    type Error = SubscriptionError;
    type Response = bool;

    fn validate(&self) -> Result<(), PayloadError> {
        let batch_size = self.subscriptions.len();

        if batch_size == 0 {
            return Err(PayloadError::BatchEmpty);
        }

        if batch_size > MAX_SUBSCRIPTION_BATCH_SIZE {
            return Err(PayloadError::BatchLimitExceeded);
        }

        for sub in &self.subscriptions {
            sub.validate()?;
        }

        Ok(())
    }

    fn into_params(self) -> Params {
        Params::BatchUnsubscribe(self)
    }
}

/// Data structure representing batch fetch request params.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BatchFetchMessages {
    /// The topics of the messages to fetch.
    pub topics: Vec<Topic>,
}

impl ServiceRequest for BatchFetchMessages {
    type Error = GenericError;
    type Response = FetchResponse;

    fn validate(&self) -> Result<(), PayloadError> {
        let batch_size = self.topics.len();

        if batch_size == 0 {
            return Err(PayloadError::BatchEmpty);
        }

        if batch_size > MAX_FETCH_BATCH_SIZE {
            return Err(PayloadError::BatchLimitExceeded);
        }

        for topic in &self.topics {
            topic.decode().map_err(|_| PayloadError::InvalidTopic)?;
        }

        Ok(())
    }

    fn into_params(self) -> Params {
        Params::BatchFetchMessages(self)
    }
}

/// Represents a message receipt.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Receipt {
    /// The topic of the message to acknowledge.
    pub topic: Topic,

    /// The ID of the message to acknowledge.
    pub message_id: MessageId,
}

/// Data structure representing publish request params.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BatchReceiveMessages {
    /// The receipts to acknowledge.
    pub receipts: Vec<Receipt>,
}

impl ServiceRequest for BatchReceiveMessages {
    type Error = GenericError;
    type Response = bool;

    fn validate(&self) -> Result<(), PayloadError> {
        let batch_size = self.receipts.len();

        if batch_size == 0 {
            return Err(PayloadError::BatchEmpty);
        }

        if batch_size > MAX_RECEIVE_BATCH_SIZE {
            return Err(PayloadError::BatchLimitExceeded);
        }

        for receipt in &self.receipts {
            receipt
                .topic
                .decode()
                .map_err(|_| PayloadError::InvalidTopic)?;
        }

        Ok(())
    }

    fn into_params(self) -> Params {
        Params::BatchReceiveMessages(self)
    }
}

/// Data structure representing publish request params.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Publish {
    /// Topic to publish to.
    pub topic: Topic,

    /// Message to publish.
    pub message: Arc<str>,

    #[serde(default, skip_serializing_if = "is_default")]
    pub attestation: Option<Arc<str>>,

    /// Duration for which the message should be kept in the mailbox if it can't
    /// be delivered, in seconds.
    #[serde(rename = "ttl")]
    pub ttl_secs: u32,

    /// A label that identifies what type of message is sent based on the RPC
    /// method used.
    pub tag: u32,

    /// A flag that identifies whether the server should trigger a notification
    /// webhook to a client through a push server.
    #[serde(default, skip_serializing_if = "is_default")]
    pub prompt: bool,
}

impl Publish {
    /// Converts these publish params into subscription params.
    pub fn as_subscription(
        &self,
        subscription_id: SubscriptionId,
        published_at: i64,
    ) -> Subscription {
        Subscription {
            id: subscription_id,
            data: SubscriptionData {
                topic: self.topic.clone(),
                message: self.message.clone(),
                attestation: self.attestation.clone(),
                published_at,
                tag: self.tag,
            },
        }
    }

    /// Creates a subscription request from these publish params.
    pub fn as_subscription_request(
        &self,
        message_id: MessageId,
        subscription_id: SubscriptionId,
        published_at: i64,
    ) -> Request {
        Request {
            id: message_id,
            jsonrpc: JSON_RPC_VERSION.clone(),
            params: Params::Subscription(self.as_subscription(subscription_id, published_at)),
        }
    }
}

#[derive(Debug, thiserror::Error, strum::EnumString, strum::IntoStaticStr, PartialEq, Eq)]
pub enum PublishError {
    #[error("TTL too short")]
    TtlTooShort,

    #[error("TTL too long")]
    TtlTooLong,

    #[error("Mailbox limit exceeded")]
    MailboxLimitExceeded,
}

impl ServiceRequest for Publish {
    type Error = PublishError;
    type Response = bool;

    fn validate(&self) -> Result<(), PayloadError> {
        self.topic
            .decode()
            .map_err(|_| PayloadError::InvalidTopic)?;

        Ok(())
    }

    fn into_params(self) -> Params {
        Params::Publish(self)
    }
}

fn is_default<T>(x: &T) -> bool
where
    T: Default + PartialEq + 'static,
{
    *x == Default::default()
}

#[derive(Debug, thiserror::Error, strum::EnumString, strum::IntoStaticStr, PartialEq, Eq)]
pub enum GenericError {
    #[error("Unknown error")]
    Unknown,
}

#[derive(Debug, thiserror::Error, strum::EnumString, strum::IntoStaticStr, PartialEq, Eq)]
pub enum WatchError {
    #[error("Invalid TTL")]
    InvalidTtl,

    #[error("Service URL is invalid or too long")]
    InvalidServiceUrl,

    #[error("Webhook URL is invalid or too long")]
    InvalidWebhookUrl,

    #[error("Invalid action")]
    InvalidAction,

    #[error("Invalid JWT")]
    InvalidJwt,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchRegisterResponse {
    /// The Relay's public key (did:key).
    pub relay_id: DidKey,
}

/// Data structure representing watch registration request params.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchRegister {
    /// JWT with [`watch::WatchRegisterClaims`] payload.
    pub register_auth: String,
}

impl ServiceRequest for WatchRegister {
    type Error = WatchError;
    type Response = WatchRegisterResponse;

    fn validate(&self) -> Result<(), PayloadError> {
        Ok(())
    }

    fn into_params(self) -> Params {
        Params::WatchRegister(self)
    }
}

/// Data structure representing watch unregistration request params.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchUnregister {
    /// JWT with [`watch::WatchUnregisterClaims`] payload.
    pub unregister_auth: String,
}

impl ServiceRequest for WatchUnregister {
    type Error = WatchError;
    type Response = bool;

    fn validate(&self) -> Result<(), PayloadError> {
        Ok(())
    }

    fn into_params(self) -> Params {
        Params::WatchUnregister(self)
    }
}

/// Data structure representing subscription request params.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Subscription {
    /// The id of the subscription.
    pub id: SubscriptionId,

    /// The published data.
    pub data: SubscriptionData,
}

impl ServiceRequest for Subscription {
    type Error = GenericError;
    type Response = bool;

    fn validate(&self) -> Result<(), PayloadError> {
        self.id
            .decode()
            .map_err(|_| PayloadError::InvalidSubscriptionId)?;

        self.data
            .topic
            .decode()
            .map_err(|_| PayloadError::InvalidTopic)?;

        Ok(())
    }

    fn into_params(self) -> Params {
        Params::Subscription(self)
    }
}

/// Data structure representing subscription message params.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionData {
    /// The topic of the subscription.
    pub topic: Topic,

    /// The message for the subscription.
    pub message: Arc<str>,

    #[serde(default, skip_serializing_if = "is_default")]
    pub attestation: Option<Arc<str>>,

    /// Message publish timestamp in UTC milliseconds.
    pub published_at: i64,

    /// A label that identifies what type of message is sent based on the RPC
    /// method used.
    #[serde(default, skip_serializing_if = "is_default")]
    pub tag: u32,
}

/// Enum representing parameters of all possible RPC requests.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum Params {
    /// Parameters to subscribe.
    #[serde(rename = "irn_subscribe", alias = "iridium_subscribe")]
    Subscribe(Subscribe),

    /// Parameters to blocking subscribe.
    #[serde(rename = "irn_subscribeBlocking", alias = "iridium_subscribeBlocking")]
    SubscribeBlocking(SubscribeBlocking),

    /// Parameters to unsubscribe.
    #[serde(rename = "irn_unsubscribe", alias = "iridium_unsubscribe")]
    Unsubscribe(Unsubscribe),

    /// Parameters to fetch.
    #[serde(rename = "irn_fetchMessages", alias = "iridium_fetchMessages")]
    FetchMessages(FetchMessages),

    /// Parameters to batch subscribe.
    #[serde(rename = "irn_batchSubscribe", alias = "iridium_batchSubscribe")]
    BatchSubscribe(BatchSubscribe),

    /// Parameters to blocking batch subscribe.
    #[serde(
        rename = "irn_batchSubscribeBlocking",
        alias = "iridium_batchSubscribeBlocking"
    )]
    BatchSubscribeBlocking(BatchSubscribeBlocking),

    /// Parameters to batch unsubscribe.
    #[serde(rename = "irn_batchUnsubscribe", alias = "iridium_batchUnsubscribe")]
    BatchUnsubscribe(BatchUnsubscribe),

    /// Parameters to batch fetch.
    #[serde(
        rename = "irn_batchFetchMessages",
        alias = "iridium_batchFetchMessages"
    )]
    BatchFetchMessages(BatchFetchMessages),

    /// Parameters to publish.
    #[serde(rename = "irn_publish", alias = "iridium_publish")]
    Publish(Publish),

    /// Parameters to batch receive.
    #[serde(rename = "irn_batchReceive", alias = "iridium_batchReceive")]
    BatchReceiveMessages(BatchReceiveMessages),

    /// Parameters to watch register.
    #[serde(rename = "irn_watchRegister", alias = "iridium_watchRegister")]
    WatchRegister(WatchRegister),

    /// Parameters to watch unregister.
    #[serde(rename = "irn_watchUnregister", alias = "iridium_watchUnregister")]
    WatchUnregister(WatchUnregister),

    /// Parameters for a subscription. The messages for any given topic sent to
    /// clients are wrapped into this format. A `publish` message to a topic
    /// results in a `subscription` message to each client subscribed to the
    /// topic the data is published for.
    #[serde(rename = "irn_subscription", alias = "iridium_subscription")]
    Subscription(Subscription),
}

/// Data structure representing a JSON RPC request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// ID this message corresponds to.
    pub id: MessageId,

    /// The JSON RPC version.
    pub jsonrpc: Arc<str>,

    /// The parameters required to fulfill this request.
    #[serde(flatten)]
    pub params: Params,
}

impl Request {
    /// Create a new instance.
    pub fn new(id: MessageId, params: Params) -> Self {
        Self {
            id,
            jsonrpc: JSON_RPC_VERSION_STR.into(),
            params,
        }
    }

    /// Validates the request payload.
    pub fn validate(&self) -> Result<(), PayloadError> {
        if !self.id.validate() {
            return Err(PayloadError::InvalidRequestId);
        }

        if self.jsonrpc.as_ref() != JSON_RPC_VERSION_STR {
            return Err(PayloadError::InvalidJsonRpcVersion);
        }

        match &self.params {
            Params::Subscribe(params) => params.validate(),
            Params::SubscribeBlocking(params) => params.validate(),
            Params::Unsubscribe(params) => params.validate(),
            Params::FetchMessages(params) => params.validate(),
            Params::BatchSubscribe(params) => params.validate(),
            Params::BatchSubscribeBlocking(params) => params.validate(),
            Params::BatchUnsubscribe(params) => params.validate(),
            Params::BatchFetchMessages(params) => params.validate(),
            Params::Publish(params) => params.validate(),
            Params::BatchReceiveMessages(params) => params.validate(),
            Params::WatchRegister(params) => params.validate(),
            Params::WatchUnregister(params) => params.validate(),
            Params::Subscription(params) => params.validate(),
        }
    }
}
