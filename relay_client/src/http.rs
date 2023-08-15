use {
    crate::{
        error::{BoxError, Error},
        ConnectionOptions,
        MessageIdGenerator,
    },
    http::{HeaderMap, StatusCode},
    relay_rpc::{
        auth::ed25519_dalek::Keypair,
        domain::{DecodedClientId, SubscriptionId, Topic},
        jwt::{self, JwtError, VerifyableClaims},
        rpc::{self, Receipt, RequestPayload},
    },
    std::{sync::Arc, time::Duration},
    url::Url,
};

pub type TransportError = reqwest::Error;
pub type Response<T> = Result<<T as RequestPayload>::Response, Error>;
pub type EmptyResponse = Result<(), Error>;

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

    #[error("Invalid HTTP status: {0}, body: {1}")]
    InvalidHttpCode(StatusCode, String),

    #[error("JWT error: {0}")]
    Jwt(#[from] JwtError),

    #[error("RPC error: code={} message={}", .0.code, .0.message)]
    RpcError(rpc::ErrorData),
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
    pub fn new(opts: &ConnectionOptions) -> Result<Self, Error> {
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
        tag: u32,
        ttl: Duration,
        prompt: bool,
    ) -> EmptyResponse {
        let ttl_secs = ttl
            .as_secs()
            .try_into()
            .map_err(|_| HttpClientError::InvalidRequest(RequestParamsError::InvalidTtl.into()))?;

        self.request(rpc::Publish {
            topic,
            message: message.into(),
            ttl_secs,
            tag,
            prompt,
        })
        .await
        .map(|_| ())
    }

    /// Subscribes on topic to receive messages.
    pub async fn subscribe(&self, topic: Topic) -> Response<rpc::Subscribe> {
        self.request(rpc::Subscribe { topic }).await
    }

    /// Unsubscribes from a topic.
    pub async fn unsubscribe(
        &self,
        topic: Topic,
        subscription_id: SubscriptionId,
    ) -> Response<rpc::Unsubscribe> {
        self.request(rpc::Unsubscribe {
            topic,
            subscription_id,
        })
        .await
    }

    /// Fetch mailbox messages for a specific topic.
    pub async fn fetch(&self, topic: Topic) -> Response<rpc::FetchMessages> {
        self.request(rpc::FetchMessages { topic }).await
    }

    /// Registers a webhook to watch messages.
    pub async fn watch_register(
        &self,
        request: WatchRegisterRequest,
        keypair: &Keypair,
    ) -> Response<rpc::WatchRegister> {
        let iat = chrono::Utc::now().timestamp();
        let ttl_sec: i64 = request
            .ttl
            .as_secs()
            .try_into()
            .map_err(|err| HttpClientError::InvalidRequest(Box::new(err)))?;
        let exp = iat + ttl_sec;

        let claims = rpc::WatchRegisterClaims {
            basic: jwt::JwtBasicClaims {
                iss: DecodedClientId::from_key(&keypair.public_key()).into(),
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
            register_auth: claims.encode(keypair).map_err(HttpClientError::Jwt)?,
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
        keypair: &Keypair,
    ) -> Response<rpc::WatchUnregister> {
        let iat = chrono::Utc::now().timestamp();

        let claims = rpc::WatchUnregisterClaims {
            basic: jwt::JwtBasicClaims {
                iss: DecodedClientId::from_key(&keypair.public_key()).into(),
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
            unregister_auth: claims.encode(keypair).map_err(HttpClientError::Jwt)?,
        };

        self.request(payload).await
    }

    /// Subscribes on multiple topics to receive messages.
    pub async fn batch_subscribe(
        &self,
        topics: impl Into<Vec<Topic>>,
    ) -> Response<rpc::BatchSubscribe> {
        self.request(rpc::BatchSubscribe {
            topics: topics.into(),
        })
        .await
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
        T: RequestPayload,
    {
        let payload = rpc::Payload::Request(rpc::Request {
            id: self.id_generator.next(),
            jsonrpc: rpc::JSON_RPC_VERSION.clone(),
            params: payload.into_params(),
        });

        let result = self
            .client
            .post(self.url.clone())
            .json(&payload)
            .send()
            .await
            .map_err(HttpClientError::Transport)?;

        let status = result.status();

        if !status.is_success() {
            let body = match result.text().await {
                Ok(body) => body,
                Err(e) => format!("... error calling result.text(): {e:?}"),
            };
            return Err(HttpClientError::InvalidHttpCode(status, body).into());
        }

        let response = result
            .json::<rpc::Payload>()
            .await
            .map_err(|_| HttpClientError::InvalidResponse)?;

        match response {
            rpc::Payload::Response(rpc::Response::Success(response)) => {
                serde_json::from_value(response.result)
                    .map_err(|_| HttpClientError::InvalidResponse.into())
            }

            rpc::Payload::Response(rpc::Response::Error(response)) => {
                Err(HttpClientError::RpcError(response.error).into())
            }

            _ => Err(HttpClientError::InvalidResponse.into()),
        }
    }
}
