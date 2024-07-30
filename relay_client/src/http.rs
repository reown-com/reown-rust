use {
    crate::{
        error::{BoxError, ClientError, Error},
        ConnectionOptions,
        MessageIdGenerator,
    },
    http::{HeaderMap, StatusCode},
    relay_rpc::{
        auth::ed25519_dalek::SigningKey,
        domain::{DecodedClientId, SubscriptionId, Topic},
        jwt::{self, JwtError, VerifyableClaims},
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

    #[error("JWT error: {0}")]
    Jwt(#[from] JwtError),
}

#[derive(Debug, Clone)]
pub struct WatchRegisterRequest {
    /// Service URL.
    pub service_url: String,
    /// Webhook URL.
    pub webhook_url: String,
    /// Watcher type. Either subscriber or publisher.
    pub watch_type: rpc::WatchType,
    /// Array of message tags to watch.
    pub tags: Vec<u32>,
    /// Array of statuses to watch.
    pub statuses: Vec<rpc::WatchStatus>,
    /// TTL for the registration.
    pub ttl: Duration,
}

#[derive(Debug, Clone)]
pub struct WatchUnregisterRequest {
    /// Service URL.
    pub service_url: String,
    /// Webhook URL.
    pub webhook_url: String,
    /// Watcher type. Either subscriber or publisher.
    pub watch_type: rpc::WatchType,
}

/// The Relay HTTP RPC client.
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    url: Url,
    origin: String,
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
        let origin = url.origin().unicode_serialization();
        let id_generator = MessageIdGenerator::new();

        Ok(Self {
            client,
            url,
            origin,
            id_generator,
        })
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

    /// Registers a webhook to watch messages.
    pub async fn watch_register(
        &self,
        request: WatchRegisterRequest,
        keypair: &SigningKey,
    ) -> Response<rpc::WatchRegister> {
        let iat = chrono::Utc::now().timestamp();
        let ttl_sec: i64 = request
            .ttl
            .as_secs()
            .try_into()
            .map_err(|err| HttpClientError::InvalidRequest(Box::new(err)).into())
            .map_err(Error::Client)?;
        let exp = iat + ttl_sec;

        let claims = rpc::WatchRegisterClaims {
            basic: jwt::JwtBasicClaims {
                iss: DecodedClientId::from_key(&keypair.verifying_key()).into(),
                aud: self.origin.clone(),
                iat,
                sub: request.service_url,
                exp: Some(exp),
            },
            act: rpc::WatchAction::Register,
            typ: request.watch_type,
            whu: request.webhook_url,
            tag: request.tags,
            sts: request.statuses,
        };

        let payload = rpc::WatchRegister {
            register_auth: claims
                .encode(keypair)
                .map_err(HttpClientError::Jwt)
                .map_err(ClientError::from)
                .map_err(Error::Client)?,
        };

        self.request(payload).await
    }

    /// Registers a webhook to watch messages on behalf of another client.
    pub async fn watch_register_behalf(
        &self,
        register_auth: String,
    ) -> Response<rpc::WatchRegister> {
        self.request(rpc::WatchRegister { register_auth }).await
    }

    /// Unregisters a webhook to watch messages.
    pub async fn watch_unregister(
        &self,
        request: WatchUnregisterRequest,
        keypair: &SigningKey,
    ) -> Response<rpc::WatchUnregister> {
        let iat = chrono::Utc::now().timestamp();

        let claims = rpc::WatchUnregisterClaims {
            basic: jwt::JwtBasicClaims {
                iss: DecodedClientId::from_key(&keypair.verifying_key()).into(),
                aud: self.origin.clone(),
                iat,
                sub: request.service_url,
                exp: None,
            },
            act: rpc::WatchAction::Unregister,
            typ: request.watch_type,
            whu: request.webhook_url,
        };

        let payload = rpc::WatchUnregister {
            unregister_auth: claims
                .encode(keypair)
                .map_err(HttpClientError::Jwt)
                .map_err(ClientError::from)
                .map_err(Error::Client)?,
        };

        self.request(payload).await
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

    /// Acknowledge receipt of messages from a subscribed client.
    pub async fn batch_receive(
        &self,
        receipts: impl Into<Vec<Receipt>>,
    ) -> Response<rpc::BatchReceiveMessages> {
        self.request(rpc::BatchReceiveMessages {
            receipts: receipts.into(),
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
