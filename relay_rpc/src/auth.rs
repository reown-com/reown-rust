use {
    crate::{
        domain::{AuthSubject, DecodedClientId},
        jwt::{JwtBasicClaims, JwtHeader},
    },
    chrono::{DateTime, Utc},
    ed25519_dalek::{ed25519::signature::Signature, Keypair, Signer},
    serde::{Deserialize, Serialize},
    std::{fmt::Display, time::Duration},
};
pub use {chrono, ed25519_dalek, rand};

#[cfg(feature = "cacao")]
pub mod cacao;
pub mod did;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid duration")]
    InvalidDuration,

    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub const RELAY_WEBSOCKET_ADDRESS: &str = "wss://relay.walletconnect.com";

pub const MULTICODEC_ED25519_BASE: &str = "z";
pub const MULTICODEC_ED25519_HEADER: [u8; 2] = [237, 1];
pub const MULTICODEC_ED25519_LENGTH: usize = 32;

pub const DEFAULT_TOKEN_AUD: &str = RELAY_WEBSOCKET_ADDRESS;
pub const DEFAULT_TOKEN_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SerializedAuthToken(String);

impl Display for SerializedAuthToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<SerializedAuthToken> for String {
    fn from(value: SerializedAuthToken) -> Self {
        value.0
    }
}

#[derive(Debug, Clone)]
pub struct AuthToken {
    sub: AuthSubject,
    aud: Option<String>,
    iat: Option<DateTime<Utc>>,
    ttl: Option<Duration>,
}

impl AuthToken {
    pub fn new(sub: impl Into<AuthSubject>) -> Self {
        Self {
            sub: sub.into(),
            aud: None,
            iat: None,
            ttl: None,
        }
    }

    pub fn aud(mut self, aud: impl Into<String>) -> Self {
        self.aud = Some(aud.into());
        self
    }

    pub fn iat(mut self, iat: impl Into<Option<DateTime<Utc>>>) -> Self {
        self.iat = iat.into();
        self
    }

    pub fn ttl(mut self, ttl: impl Into<Option<Duration>>) -> Self {
        self.ttl = ttl.into();
        self
    }

    pub fn as_jwt(&self, key: &Keypair) -> Result<SerializedAuthToken, Error> {
        let iat = self.iat.unwrap_or_else(Utc::now);
        let ttl = self.ttl.unwrap_or(DEFAULT_TOKEN_TTL);
        let aud = self.aud.as_deref().unwrap_or(DEFAULT_TOKEN_AUD);

        encode_auth_token(key, self.sub.as_ref(), aud, iat, ttl)
    }
}

pub fn encode_auth_token(
    key: &Keypair,
    sub: impl Into<String>,
    aud: impl Into<String>,
    iat: DateTime<Utc>,
    ttl: Duration,
) -> Result<SerializedAuthToken, Error> {
    let encoder = &data_encoding::BASE64URL_NOPAD;
    let exp = iat + chrono::Duration::from_std(ttl).map_err(|_| Error::InvalidDuration)?;

    let claims = {
        let data = JwtBasicClaims {
            iss: DecodedClientId::from_key(&key.public_key()).into(),
            sub: sub.into(),
            aud: aud.into(),
            iat: iat.timestamp(),
            exp: Some(exp.timestamp()),
        };

        encoder.encode(serde_json::to_string(&data)?.as_bytes())
    };

    let header = encoder.encode(serde_json::to_string(&JwtHeader::default())?.as_bytes());
    let message = format!("{header}.{claims}");

    let signature = {
        let data = key.sign(message.as_bytes());

        encoder.encode(data.as_bytes())
    };

    Ok(SerializedAuthToken(format!("{message}.{signature}")))
}
