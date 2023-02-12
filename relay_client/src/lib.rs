pub use {client::*, errors::*};
use {
    relay_rpc::{
        auth::{SerializedAuthToken, RELAY_WEBSOCKET_ADDRESS},
        domain::ProjectId,
        user_agent::UserAgent,
    },
    serde::Serialize,
    tokio_tungstenite::tungstenite::{client::IntoClientRequest, http},
};

mod client;
mod errors;

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
        }
    }

    pub fn with_address(mut self, address: impl Into<String>) -> Self {
        self.address = address.into();
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

    fn into_request(self) -> Result<http::Request<()>, Error> {
        let ConnectionOptions {
            address,
            project_id,
            auth,
            origin,
            user_agent,
        } = self;

        let query = {
            let auth = if let Authorization::Query(auth) = &auth {
                Some(auth.to_owned())
            } else {
                None
            };

            #[derive(Serialize)]
            #[serde(rename_all = "camelCase")]
            struct QueryParams {
                project_id: ProjectId,
                auth: Option<SerializedAuthToken>,
                ua: Option<UserAgent>,
            }

            let query = QueryParams {
                project_id,
                auth,
                ua: user_agent,
            };

            serde_qs::to_string(&query).map_err(RequestBuildError::Query)?
        };

        let mut request = format!("{address}/?{query}")
            .into_client_request()
            .map_err(RequestBuildError::Other)?;

        let headers = request.headers_mut();

        if let Authorization::Header(token) = &auth {
            let value = format!("Bearer {token}")
                .parse()
                .map_err(|_| RequestBuildError::Headers)?;

            headers.append("Authorization", value);
        }

        if let Some(origin) = &origin {
            let value = origin.parse().map_err(|_| RequestBuildError::Headers)?;

            headers.append("Origin", value);
        }

        Ok(request)
    }
}
