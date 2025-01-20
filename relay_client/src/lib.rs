use {
    crate::error::{ClientError, RequestBuildError},
    ::http::HeaderMap,
    relay_rpc::{
        auth::{SerializedAuthToken, RELAY_WEBSOCKET_ADDRESS},
        domain::{MessageId, ProjectId, SubscriptionId},
        rpc::{SubscriptionError, SubscriptionResult},
        user_agent::UserAgent,
    },
    serde::Serialize,
    std::sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    url::Url,
};

pub mod error;
pub mod http;
pub mod websocket;

pub type HttpRequest<T> = ::http::Request<T>;

/// Relay authorization method. A wrapper around [`SerializedAuthToken`].
#[derive(Debug, Clone)]
pub enum Authorization {
    /// Uses query string to pass the auth token, e.g. `?auth=<token>`.
    Query(SerializedAuthToken),

    /// Uses the `Authorization: Bearer <token>` HTTP header.
    Header(SerializedAuthToken),
}

/// Relay connection options.
#[derive(Debug, Clone)]
pub struct ConnectionOptions {
    /// The Relay websocket address. The default address is
    /// `wss://relay.walletconnect.com`.
    pub address: String,

    /// The project-specific secret key. Can be generated in the Cloud Dashboard
    /// at the following URL: <https://cloud.walletconnect.com/app>
    pub project_id: ProjectId,

    /// The authorization method and auth token to use.
    pub auth: Authorization,

    /// Optional origin of the request. Subject to allow-list validation.
    pub origin: Option<String>,

    /// Optional package name. Used instead of `origin` for allow-list
    /// validation.
    pub package_name: Option<String>,

    /// Optional bundle ID. Used instead of `origin` for allow-list validation.
    pub bundle_id: Option<String>,

    /// Optional user agent parameters.
    pub user_agent: Option<UserAgent>,
}

impl ConnectionOptions {
    pub fn new(project_id: impl Into<ProjectId>, auth: SerializedAuthToken) -> Self {
        Self {
            address: RELAY_WEBSOCKET_ADDRESS.into(),
            project_id: project_id.into(),
            auth: Authorization::Query(auth),
            origin: None,
            user_agent: None,
            package_name: None,
            bundle_id: None,
        }
    }

    pub fn with_address(mut self, address: impl Into<String>) -> Self {
        self.address = address.into();
        self
    }

    pub fn with_package_name(mut self, package_name: impl Into<String>) -> Self {
        self.package_name = Some(package_name.into());
        self
    }

    pub fn with_bundle_id(mut self, bundle_id: impl Into<String>) -> Self {
        self.bundle_id = Some(bundle_id.into());
        self
    }

    pub fn with_origin(mut self, origin: impl Into<Option<String>>) -> Self {
        self.origin = origin.into();
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<Option<UserAgent>>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    pub fn as_url(&self) -> Result<Url, RequestBuildError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct QueryParams<'a> {
            project_id: &'a ProjectId,
            auth: Option<&'a SerializedAuthToken>,
            ua: Option<&'a UserAgent>,
            package_name: Option<&'a str>,
            bundle_id: Option<&'a str>,
        }

        let query = serde_qs::to_string(&QueryParams {
            project_id: &self.project_id,
            auth: if let Authorization::Query(auth) = &self.auth {
                Some(auth)
            } else {
                None
            },
            ua: self.user_agent.as_ref(),
            package_name: self.package_name.as_deref(),
            bundle_id: self.bundle_id.as_deref(),
        })
        .map_err(RequestBuildError::Query)?;

        let mut url = Url::parse(&self.address).map_err(RequestBuildError::Url)?;
        url.set_query(Some(&query));

        Ok(url)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn as_ws_request(&self) -> Result<HttpRequest<()>, RequestBuildError> {
        use {
            crate::websocket::WebsocketClientError,
            tokio_tungstenite::tungstenite::client::IntoClientRequest,
        };

        let url = self.as_url()?;

        let mut request = url
            .into_client_request()
            .map_err(WebsocketClientError::Transport)?;

        self.update_request_headers(request.headers_mut())?;

        Ok(request)
    }

    #[cfg(target_arch = "wasm32")]
    fn as_ws_request(&self) -> Result<HttpRequest<()>, RequestBuildError> {
        use crate::websocket::WebsocketClientError;

        let url = self.as_url()?;
        let mut request = HttpRequest::builder()
            .uri(format!("{}", url))
            .body(())
            .map_err(WebsocketClientError::HttpErr)?;

        self.update_request_headers(request.headers_mut())?;
        Ok(request)
    }

    fn update_request_headers(&self, headers: &mut HeaderMap) -> Result<(), RequestBuildError> {
        if let Authorization::Header(token) = &self.auth {
            let value = format!("Bearer {token}")
                .parse()
                .map_err(|_| RequestBuildError::Headers)?;

            headers.append("Authorization", value);
        }

        if let Some(origin) = &self.origin {
            let value = origin.parse().map_err(|_| RequestBuildError::Headers)?;

            headers.append("Origin", value);
        }

        Ok(())
    }
}

/// Generates unique message IDs for use in RPC requests. Uses 56 bits for the
/// timestamp with millisecond precision, with the last 8 bits from a monotonic
/// counter. Capable of producing up to `256000` unique values per second.
#[derive(Debug, Clone)]
pub struct MessageIdGenerator {
    next: Arc<AtomicU8>,
}

impl MessageIdGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Generates a [`MessageId`].
    pub fn next(&self) -> MessageId {
        let next = self.next.fetch_add(1, Ordering::Relaxed) as u64;
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let id = (timestamp << 8) | next;

        MessageId::new(id)
    }
}

impl Default for MessageIdGenerator {
    fn default() -> Self {
        Self {
            next: Arc::new(AtomicU8::new(0)),
        }
    }
}

#[inline]
fn convert_subscription_result(
    res: SubscriptionResult,
) -> Result<SubscriptionId, error::Error<SubscriptionError>> {
    match res {
        SubscriptionResult::Id(id) => Ok(id),
        SubscriptionResult::Error(err) => Err(ClientError::from(err).into()),
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        std::{collections::HashSet, hash::Hash},
    };

    fn elements_unique<T>(iter: T) -> bool
    where
        T: IntoIterator,
        T::Item: Eq + Hash,
    {
        let mut set = HashSet::new();
        iter.into_iter().all(move |x| set.insert(x))
    }

    #[test]
    fn unique_message_ids() {
        let gen = MessageIdGenerator::new();
        // N.B. We can produce up to 256 unique values within 1ms.
        let values = (0..256).map(move |_| gen.next()).collect::<Vec<_>>();
        assert!(elements_unique(values));
    }
}
