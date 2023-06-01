use {
    crate::{
        error::{BoxError, Error},
        ConnectionOptions,
        MessageIdGenerator,
    },
    http::{HeaderMap, StatusCode},
    relay_rpc::{
        auth::ed25519_dalek::Keypair,
        domain::DecodedClientId,
        jwt::{self, JwtError, VerifyableClaims},
        rpc::{self, RequestPayload},
    },
    std::time::Duration,
    url::Url,
};

pub type TransportError = reqwest::Error;

#[derive(Debug, thiserror::Error)]
pub enum HttpClientError {
    #[error("HTTP transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("Invalid request: {0}")]
    InvalidRequest(BoxError),

    #[error("Invalid response")]
    InvalidResponse,

    #[error("Invalid HTTP status: {}", .0.as_u16())]
    InvalidHttpCode(StatusCode),

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

    pub async fn watch_register(
        &self,
        request: WatchRegisterRequest,
        keypair: &Keypair,
    ) -> Result<<rpc::WatchRegister as RequestPayload>::Response, Error> {
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

    pub async fn watch_unregister(
        &self,
        request: WatchUnregisterRequest,
        keypair: &Keypair,
    ) -> Result<<rpc::WatchUnregister as RequestPayload>::Response, Error> {
        let iat = chrono::Utc::now().timestamp();

        let claims = rpc::WatchUnregisterClaims {
            basic: jwt::JwtBasicClaims {
                iss: DecodedClientId::from_key(&keypair.public_key()).into(),
                aud: self.origin.clone(),
                iat,
                sub: request.service_url,
                exp: Some(iat + 60 * 60),
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

    pub async fn request<T>(&self, payload: T) -> Result<T::Response, Error>
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
            return Err(HttpClientError::InvalidHttpCode(status).into());
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
