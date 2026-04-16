use {
    crate::{
        error::{BoxError, ClientError, Error},
        ConnectionOptions,
        MessageIdGenerator,
    },
    http::{HeaderMap, StatusCode},
    relay_rpc::{
        domain::{SubscriptionId, Topic},
        rpc::{self, Receipt, ServiceRequest},
    },
    std::{sync::Arc, time::Duration},
    url::Url,
};

pub type TransportError = reqwest::Error;
pub type Response<T> = Result<<T as ServiceRequest>::Response, Error<<T as ServiceRequest>::Error>>;
pub type EmptyResponse<T> = Result<(), Error<<T as ServiceRequest>::Error>>;

#[derive(Debug, thiserror::Error)]
pub enum RequestParamsError {
    #[error("Invalid TTL")]
    InvalidTtl,
}

#[derive(Debug, thiserror::Error)]
pub enum HttpClientError {
    #[error("HTTP transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("Invalid request: {0}")]
    InvalidRequest(BoxError),

    #[error("Invalid response")]
    InvalidResponse,

    #[error("Invalid HTTP status: {0}, body: {1:?}")]
    InvalidHttpCode(StatusCode, reqwest::Result<String>),
}

/// The Relay HTTP RPC client.
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    url: Url,
    id_generator: MessageIdGenerator,
}

impl Client {
    pub fn new(opts: &ConnectionOptions) -> Result<Self, ClientError> {
        let mut headers = HeaderMap::new();
        opts.update_request_headers(&mut headers)?;

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(HttpClientError::Transport)?;

        let url = opts.as_url()?;
        let id_generator = MessageIdGenerator::new();

        Ok(Self {
            client,
            url,
            id_generator,
        })
    }

    pub async fn propose_session(
        &self,
        pairing_topic: Topic,
        session_proposal: impl Into<Arc<str>>,
        attestation: impl Into<Option<Arc<str>>>,
        analytics: Option<rpc::AnalyticsData>,
    ) -> Response<rpc::ProposeSession> {
        self.request(rpc::ProposeSession {
            pairing_topic,
            session_proposal: session_proposal.into(),
            attestation: attestation.into(),
            analytics: analytics.map(Into::into),
        })
        .await
    }

    pub async fn approve_session(
        &self,
        pairing_topic: Topic,
        session_topic: Topic,
        session_proposal_response: impl Into<Arc<str>>,
        session_settlement_request: impl Into<Arc<str>>,
        properties: rpc::SessionProperties,
        analytics: Option<rpc::AnalyticsData>,
    ) -> Response<rpc::ApproveSession> {
        self.request(rpc::ApproveSession {
            pairing_topic,
            session_topic,
            session_proposal_response: session_proposal_response.into(),
            session_settlement_request: session_settlement_request.into(),
            properties: Arc::new(properties),
            analytics: analytics.map(Into::into),
        })
        .await
    }

    /// Publishes a message over the network on given topic.
    pub async fn publish(
        &self,
        topic: Topic,
        message: impl Into<Arc<str>>,
        attestation: impl Into<Option<Arc<str>>>,
        tag: u32,
        ttl: Duration,
        prompt: bool,
    ) -> EmptyResponse<rpc::Publish> {
        let ttl_secs = ttl
            .as_secs()
            .try_into()
            .map_err(|_| {
                HttpClientError::InvalidRequest(RequestParamsError::InvalidTtl.into()).into()
            })
            .map_err(Error::Client)?;

        self.request(rpc::Publish {
            topic,
            message: message.into(),
            attestation: attestation.into(),
            ttl_secs,
            tag,
            prompt,
            analytics: None,
        })
        .await
        .map(|_| ())
    }

    /// Subscribes on topic to receive messages. The request is resolved
    /// optimistically as soon as the relay receives it.
    pub async fn subscribe(&self, topic: Topic) -> Response<rpc::Subscribe> {
        self.request(rpc::Subscribe { topic }).await
    }

    /// Subscribes on topic to receive messages. The request is resolved only
    /// when fully processed by the relay.
    /// Note: This function is experimental and will likely be removed in the
    /// future.
    pub async fn subscribe_blocking(&self, topic: Topic) -> Response<rpc::SubscribeBlocking> {
        self.request(rpc::SubscribeBlocking { topic }).await
    }

    /// Unsubscribes from a topic.
    pub async fn unsubscribe(&self, topic: Topic) -> Response<rpc::Unsubscribe> {
        self.request(rpc::Unsubscribe { topic }).await
    }

    /// Fetch mailbox messages for a specific topic.
    pub async fn fetch(&self, topic: Topic) -> Response<rpc::FetchMessages> {
        self.request(rpc::FetchMessages { topic }).await
    }

    /// Subscribes on multiple topics to receive messages. The request is
    /// resolved optimistically as soon as the relay receives it.
    pub async fn batch_subscribe(
        &self,
        topics: impl Into<Vec<Topic>>,
    ) -> Response<rpc::BatchSubscribe> {
        self.request(rpc::BatchSubscribe {
            topics: topics.into(),
        })
        .await
    }

    /// Subscribes on multiple topics to receive messages. The request is
    /// resolved only when fully processed by the relay.
    /// Note: This function is experimental and will likely be removed in the
    /// future.
    pub async fn batch_subscribe_blocking(
        &self,
        topics: impl Into<Vec<Topic>>,
    ) -> Result<
        Vec<Result<SubscriptionId, Error<rpc::SubscriptionError>>>,
        Error<rpc::SubscriptionError>,
    > {
        Ok(self
            .request(rpc::BatchSubscribeBlocking {
                topics: topics.into(),
            })
            .await?
            .into_iter()
            .map(crate::convert_subscription_result)
            .collect())
    }

    /// Unsubscribes from multiple topics.
    pub async fn batch_unsubscribe(
        &self,
        subscriptions: impl Into<Vec<rpc::Unsubscribe>>,
    ) -> Response<rpc::BatchUnsubscribe> {
        self.request(rpc::BatchUnsubscribe {
            subscriptions: subscriptions.into(),
        })
        .await
    }

    /// Fetch mailbox messages for multiple topics.
    pub async fn batch_fetch(
        &self,
        topics: impl Into<Vec<Topic>>,
    ) -> Response<rpc::BatchFetchMessages> {
        self.request(rpc::BatchFetchMessages {
            topics: topics.into(),
        })
        .await
    }

    pub(crate) async fn request<T>(&self, payload: T) -> Response<T>
    where
        T: ServiceRequest,
    {
        let payload = rpc::Payload::Request(rpc::Request {
            id: self.id_generator.next(),
            jsonrpc: rpc::JSON_RPC_VERSION.clone(),
            params: payload.into_params(),
        });

        let response = async {
            let result = self
                .client
                .post(self.url.clone())
                .json(&payload)
                .send()
                .await
                .map_err(HttpClientError::Transport)?;

            let status = result.status();

            if !status.is_success() {
                let body = result.text().await;
                return Err(HttpClientError::InvalidHttpCode(status, body));
            }

            result
                .json::<rpc::Payload>()
                .await
                .map_err(|_| HttpClientError::InvalidResponse)
        }
        .await
        .map_err(ClientError::from)
        .map_err(Error::Client)?;

        match response {
            rpc::Payload::Response(rpc::Response::Success(response)) => {
                serde_json::from_value(response.result)
                    .map_err(|_| Error::Client(HttpClientError::InvalidResponse.into()))
            }

            rpc::Payload::Response(rpc::Response::Error(response)) => {
                Err(ClientError::from(response.error).into())
            }

            _ => Err(Error::Client(HttpClientError::InvalidResponse.into())),
        }
    }
}
