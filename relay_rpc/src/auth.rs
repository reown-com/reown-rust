use {
    crate::domain::{AuthSubject, ClientId, ClientIdDecodingError, DecodedClientId},
    chrono::{DateTime, Utc},
    ed25519_dalek::{ed25519::signature::Signature, Keypair, Signer},
    serde::{Deserialize, Serialize},
    std::{collections::HashSet, fmt::Display, time::Duration},
};
pub use {chrono, ed25519_dalek, rand};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid duration")]
    InvalidDuration,

    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub const RELAY_WEBSOCKET_ADDRESS: &str = "wss://relay.walletconnect.com";

pub const DID_DELIMITER: &str = ":";
pub const DID_PREFIX: &str = "did";
pub const DID_METHOD: &str = "key";

pub const MULTICODEC_ED25519_BASE: &str = "z";
pub const MULTICODEC_ED25519_HEADER: [u8; 2] = [237, 1];
pub const MULTICODEC_ED25519_LENGTH: usize = 32;

pub const JWT_DELIMITER: &str = ".";
pub const JWT_HEADER_TYP: &str = "JWT";
pub const JWT_HEADER_ALG: &str = "EdDSA";
pub const JWT_VALIDATION_TIME_LEEWAY_SECS: i64 = 120;

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

#[derive(Serialize, Deserialize)]
pub struct JwtHeader<'a> {
    pub typ: &'a str,
    pub alg: &'a str,
}

impl<'a> JwtHeader<'a> {
    pub fn is_valid(&self) -> bool {
        self.typ == JWT_HEADER_TYP && self.alg == JWT_HEADER_ALG
    }
}

#[derive(Serialize, Deserialize)]
pub struct JwtClaims<'a> {
    pub iss: &'a str,
    pub sub: &'a str,
    pub aud: &'a str,
    pub iat: i64,
    pub exp: i64,
}

impl<'a> JwtClaims<'a> {
    pub fn validate(
        &self,
        aud: &HashSet<String>,
        time_leeway: impl Into<Option<i64>>,
    ) -> Result<(), JwtVerificationError> {
        let time_leeway = time_leeway
            .into()
            .unwrap_or(JWT_VALIDATION_TIME_LEEWAY_SECS);
        let now = Utc::now().timestamp();

        if now - time_leeway > self.exp {
            return Err(JwtVerificationError::Expired);
        }

        if now + time_leeway < self.iat {
            return Err(JwtVerificationError::NotYetValid);
        }

        if !aud.contains(self.aud) {
            return Err(JwtVerificationError::InvalidAudience);
        }
        Ok(())
    }
}

pub fn encode_auth_token(
    key: &Keypair,
    sub: &str,
    aud: &str,
    iat: DateTime<Utc>,
    ttl: Duration,
) -> Result<SerializedAuthToken, Error> {
    let encoder = &data_encoding::BASE64URL_NOPAD;
    let exp = iat + chrono::Duration::from_std(ttl).map_err(|_| Error::InvalidDuration)?;

    let iss = {
        let client_id = DecodedClientId(*key.public_key().as_bytes());

        format!("{DID_PREFIX}{DID_DELIMITER}{DID_METHOD}{DID_DELIMITER}{client_id}",)
    };

    let claims = {
        let data = JwtClaims {
            iss: &iss,
            sub,
            aud,
            iat: iat.timestamp(),
            exp: exp.timestamp(),
        };

        encoder.encode(serde_json::to_string(&data)?.as_bytes())
    };

    let header = {
        let data = JwtHeader {
            typ: JWT_HEADER_TYP,
            alg: JWT_HEADER_ALG,
        };

        encoder.encode(serde_json::to_string(&data)?.as_bytes())
    };

    let message = format!("{header}.{claims}");

    let signature = {
        let data = key.sign(message.as_bytes());

        encoder.encode(data.as_bytes())
    };

    Ok(SerializedAuthToken(format!("{message}.{signature}")))
}

#[derive(Debug, thiserror::Error)]
pub enum JwtVerificationError {
    #[error("Invalid format")]
    Format,

    #[error("Invalid encoding")]
    Encoding,

    #[error("Invalid JWT signing algorithm")]
    Header,

    #[error("JWT Token is expired")]
    Expired,

    #[error("JWT Token is not yet valid")]
    NotYetValid,

    #[error("Invalid audience")]
    InvalidAudience,

    #[error("Invalid signature")]
    Signature,

    #[error("Invalid JSON")]
    Serialization,

    #[error("Invalid issuer DID prefix")]
    IssuerPrefix,

    #[error("Invalid issuer DID method")]
    IssuerMethod,

    #[error("Invalid issuer format")]
    IssuerFormat,

    #[error(transparent)]
    PubKey(#[from] ClientIdDecodingError),
}

#[derive(Debug)]
pub struct Jwt(pub String);

impl Jwt {
    pub fn decode(&self, aud: &HashSet<String>) -> Result<ClientId, JwtVerificationError> {
        let mut parts = self.0.splitn(3, JWT_DELIMITER);

        let (Some(header), Some(claims)) = (parts.next(), parts.next()) else {
            return Err(JwtVerificationError::Format);
        };

        let decoder = &data_encoding::BASE64URL_NOPAD;

        let header_len = decoder
            .decode_len(header.len())
            .map_err(|_| JwtVerificationError::Encoding)?;
        let claims_len = decoder
            .decode_len(claims.len())
            .map_err(|_| JwtVerificationError::Encoding)?;

        let mut output = vec![0u8; header_len.max(claims_len)];

        // Decode header.
        data_encoding::BASE64URL_NOPAD
            .decode_mut(header.as_bytes(), &mut output[..header_len])
            .map_err(|_| JwtVerificationError::Encoding)?;

        {
            let header = serde_json::from_slice::<JwtHeader>(&output[..header_len])
                .map_err(|_| JwtVerificationError::Serialization)?;

            if !header.is_valid() {
                return Err(JwtVerificationError::Header);
            }
        }

        // Decode claims.
        data_encoding::BASE64URL_NOPAD
            .decode_mut(claims.as_bytes(), &mut output[..claims_len])
            .map_err(|_| JwtVerificationError::Encoding)?;

        let claims = serde_json::from_slice::<JwtClaims>(&output[..claims_len])
            .map_err(|_| JwtVerificationError::Serialization)?;

        // Basic token validation: `iat`, `exp` and `aud`.
        claims.validate(aud, None)?;

        let did_key = claims
            .iss
            .strip_prefix(DID_PREFIX)
            .ok_or(JwtVerificationError::IssuerPrefix)?
            .strip_prefix(DID_DELIMITER)
            .ok_or(JwtVerificationError::IssuerFormat)?
            .strip_prefix(DID_METHOD)
            .ok_or(JwtVerificationError::IssuerMethod)?
            .strip_prefix(DID_DELIMITER)
            .ok_or(JwtVerificationError::IssuerFormat)?;

        let pub_key = did_key.parse::<DecodedClientId>()?;

        let mut parts = self.0.rsplitn(2, JWT_DELIMITER);

        let (Some(signature), Some(message)) = (parts.next(), parts.next()) else {
            return Err(JwtVerificationError::Format);
        };

        let key = jsonwebtoken::DecodingKey::from_ed_der(pub_key.as_ref());

        // Finally, verify signature.
        let sig_result = jsonwebtoken::crypto::verify(
            signature,
            message.as_bytes(),
            &key,
            jsonwebtoken::Algorithm::EdDSA,
        );

        match sig_result {
            Ok(true) => Ok(pub_key.into()),
            _ => Err(JwtVerificationError::Signature),
        }
    }
}
