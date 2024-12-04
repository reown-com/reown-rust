use {
    crate::domain::DidKey,
    chrono::Utc,
    ed25519_dalek::{Signer, SigningKey},
    serde::{de::DeserializeOwned, Deserialize, Serialize},
    std::collections::HashSet,
};

pub const JWT_DELIMITER: &str = ".";
pub const JWT_HEADER_TYP: &str = "JWT";
pub const JWT_HEADER_ALG: &str = "EdDSA";
pub const JWT_VALIDATION_TIME_LEEWAY_SECS: i64 = 120;

#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("Invalid format")]
    Format,

    #[error("Invalid encoding")]
    Encoding,

    #[error("Invalid JWT signing algorithm")]
    Header,

    #[error("JWT Token is expired: {:?}", expiration)]
    Expired { expiration: Option<i64> },

    #[error(
        "JWT Token is not yet valid: basic.iat: {}, now + time_leeway: {}, time_leeway: {}",
        basic_iat,
        now_time_leeway,
        time_leeway
    )]
    NotYetValid {
        basic_iat: i64,
        now_time_leeway: i64,
        time_leeway: i64,
    },

    #[error("Invalid audience")]
    InvalidAudience,

    #[error("Invalid signature")]
    Signature,

    #[error("Encoding keypair mismatch")]
    InvalidKeypair,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Serialize, Deserialize)]
pub struct JwtHeader<'a> {
    #[serde(borrow)]
    pub typ: &'a str,
    #[serde(borrow)]
    pub alg: &'a str,
}

impl Default for JwtHeader<'_> {
    fn default() -> Self {
        Self {
            typ: JWT_HEADER_TYP,
            alg: JWT_HEADER_ALG,
        }
    }
}

impl JwtHeader<'_> {
    pub fn is_valid(&self) -> bool {
        self.typ == JWT_HEADER_TYP && self.alg == JWT_HEADER_ALG
    }
}

/// Basic JWT claims that are common to all JWTs used by the Relay.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JwtBasicClaims {
    /// Client ID matching the watch type.
    pub iss: DidKey,
    /// Relay URL.
    pub aud: String,
    /// Service URL.
    pub sub: String,
    /// Issued at, timestamp.
    pub iat: i64,
    /// Expiration, timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
}

impl VerifyableClaims for JwtBasicClaims {
    fn basic(&self) -> &JwtBasicClaims {
        self
    }
}

pub trait VerifyableClaims: Serialize + DeserializeOwned {
    /// Returns a reference to the basic claims, which may be a part of a larger
    /// set of claims.
    fn basic(&self) -> &JwtBasicClaims;

    /// Encodes the claims into a JWT string, signing it with the provided key.
    /// Returns an error if the provided key does not match the public key in
    /// the claims (`iss`), or if serialization fails.
    fn encode(&self, key: &SigningKey) -> Result<String, JwtError> {
        let public_key = self.basic().iss.0.as_public_key();

        // Make sure the keypair matches the public key in the claims.
        if public_key != key.verifying_key() {
            return Err(JwtError::InvalidKeypair);
        }

        let encoder = &data_encoding::BASE64URL_NOPAD;
        let header = encoder.encode(serde_json::to_string(&JwtHeader::default())?.as_bytes());
        let claims = encoder.encode(serde_json::to_string(self)?.as_bytes());
        let message = format!("{header}.{claims}");
        let signature = encoder.encode(&key.sign(message.as_bytes()).to_bytes());

        Ok(format!("{message}.{signature}"))
    }

    /// Tries to parse the claims from a string, returning an error if the
    /// parsing fails for any reason.
    ///
    /// Note: This does not perorm the actual verification of the claims. After
    /// successful decoding, the claims should be verified using the
    /// [`VerifyableClaims::verify_basic()`] method.
    fn try_from_str(data: &str) -> Result<Self, JwtError>
    where
        Self: Sized,
    {
        let mut parts = data.splitn(3, JWT_DELIMITER);

        let (Some(header), Some(claims)) = (parts.next(), parts.next()) else {
            return Err(JwtError::Format);
        };

        let decoder = &data_encoding::BASE64URL_NOPAD;

        let header_len = decoder
            .decode_len(header.len())
            .map_err(|_| JwtError::Encoding)?;
        let claims_len = decoder
            .decode_len(claims.len())
            .map_err(|_| JwtError::Encoding)?;

        let mut output = vec![0u8; header_len.max(claims_len)];

        // Decode header.
        data_encoding::BASE64URL_NOPAD
            .decode_mut(
                header.as_bytes(),
                output.get_mut(..header_len).ok_or(JwtError::Encoding)?,
            )
            .map_err(|_| JwtError::Encoding)?;

        {
            let header = serde_json::from_slice::<JwtHeader>(
                output.get(..header_len).ok_or(JwtError::Encoding)?,
            )
            .map_err(JwtError::Serialization)?;

            if !header.is_valid() {
                return Err(JwtError::Header);
            }
        }

        // Decode claims.
        data_encoding::BASE64URL_NOPAD
            .decode_mut(
                claims.as_bytes(),
                output.get_mut(..claims_len).ok_or(JwtError::Encoding)?,
            )
            .map_err(|_| JwtError::Encoding)?;

        let claims =
            serde_json::from_slice::<Self>(output.get(..claims_len).ok_or(JwtError::Encoding)?)
                .map_err(JwtError::Serialization)?;

        let mut parts = data.rsplitn(2, JWT_DELIMITER);

        let (Some(signature), Some(message)) = (parts.next(), parts.next()) else {
            return Err(JwtError::Format);
        };

        let key = jsonwebtoken::DecodingKey::from_ed_der(claims.basic().iss.as_ref());

        // Finally, verify signature.
        let sig_result = jsonwebtoken::crypto::verify(
            signature,
            message.as_bytes(),
            &key,
            jsonwebtoken::Algorithm::EdDSA,
        );

        match sig_result {
            Ok(true) => Ok(claims),

            _ => Err(JwtError::Signature),
        }
    }

    /// Performs basic verification of the claims. This includes the following
    /// checks:
    /// - The token is not expired (with a configurable leeway). This is
    ///   optional if the token has an `exp` value;
    /// - The token is not used before it's valid;
    /// - The token is issued for the correct audience.
    fn verify_basic(
        &self,
        aud: &HashSet<String>,
        time_leeway: impl Into<Option<i64>>,
    ) -> Result<(), JwtError> {
        let basic = self.basic();
        let time_leeway = time_leeway
            .into()
            .unwrap_or(JWT_VALIDATION_TIME_LEEWAY_SECS);
        let now = Utc::now().timestamp();

        if matches!(basic.exp, Some(exp) if now - time_leeway > exp) {
            return Err(JwtError::Expired {
                expiration: basic.exp,
            });
        }

        if now + time_leeway < basic.iat {
            return Err(JwtError::NotYetValid {
                basic_iat: basic.iat,
                now_time_leeway: now + time_leeway,
                time_leeway,
            });
        }

        if !aud.contains(&basic.aud) {
            return Err(JwtError::InvalidAudience);
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use {
        crate::{
            auth::AuthToken,
            domain::ClientId,
            jwt::{JwtBasicClaims, JwtError, VerifyableClaims, JWT_VALIDATION_TIME_LEEWAY_SECS},
        },
        ed25519_dalek::SigningKey,
        rand::rngs::OsRng,
        std::{collections::HashSet, time::Duration},
    };

    #[derive(Debug)]
    pub struct Jwt(pub String);

    impl Jwt {
        pub fn decode(&self, aud: &HashSet<String>) -> Result<ClientId, JwtError> {
            let claims = JwtBasicClaims::try_from_str(&self.0)?;
            claims.verify_basic(aud, None)?;
            Ok(claims.iss.into())
        }
    }

    #[test]
    fn token_validation() {
        let aud = HashSet::from(["wss://relay.walletconnect.com".to_owned()]);

        // Invalid signature.
        let jwt = Jwt("eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJkaWQ6a2V5Ono2TWtvZEhad25lVlJTaHRhTGY4SktZa3hwREdwMXZHWm5wR21kQnBYOE0yZXh4SCIsInN1YiI6ImM0NzlmZTVkYzQ2NGU3NzFlNzhiMTkzZDIzOWE2NWI1OGQyNzhjYWQxYzM0YmZiMGI1NzE2ZTViYjUxNDkyOGUiLCJhdWQiOiJ3c3M6Ly9yZWxheS53YWxsZXRjb25uZWN0LmNvbSIsImlhdCI6MTY1NjkxMDA5NywiZXhwIjo0ODEyNjcwMDk3fQ.CLryc7bGZ_mBVh-P5p2tDDkjY8m9ji9xZXixJCbLLd4TMBh7F0EkChbWOOUQp4DyXUVK4CN-hxMZgt2xnePUBAx".to_owned());
        assert!(matches!(jwt.decode(&aud), Err(JwtError::Signature)));

        // Invalid multicodec header.
        let jwt = Jwt("eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJkaWQ6a2V5Ono2TWt2eDRWVnVCQlBIekVvTERiNWdOQzRyUW1uSnN0YzFib29oS2ZjSlV0OU12NjUiLCJzdWIiOiJjNDc5ZmU1ZGM0NjRlNzcxZTc4YjE5M2QyMzlhNjViNThkMjc4Y2FkMWMzNGJmYjBiNTcxNmU1YmI1MTQ5MjhlIiwiYXVkIjoid3NzOi8vcmVsYXkud2FsbGV0Y29ubmVjdC5jb20iLCJpYXQiOjE2NTY5MTAwOTcsImV4cCI6NDgxMjY3MDA5N30.ixjxEISufsDpdsp4MRwD4Q100d8s7v4mSlIWIad6q8Nh__768pzPaCAVXQIZLxKPhuJQ92cZi7tVUJtAE1_UCg".to_owned());
        assert!(matches!(jwt.decode(&aud), Err(JwtError::Serialization(..))));

        // Invalid multicodec base.
        let jwt = Jwt("eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJkaWQ6a2V5Onh6Nk1rb2RIWnduZVZSU2h0YUxmOEpLWWt4cERHcDF2R1pucEdtZEJwWDhNMmV4eEgiLCJzdWIiOiJjNDc5ZmU1ZGM0NjRlNzcxZTc4YjE5M2QyMzlhNjViNThkMjc4Y2FkMWMzNGJmYjBiNTcxNmU1YmI1MTQ5MjhlIiwiYXVkIjoid3NzOi8vcmVsYXkud2FsbGV0Y29ubmVjdC5jb20iLCJpYXQiOjE2NTY5MTAwOTcsImV4cCI6NDgxMjY3MDA5N30.BINvB6JpUyp5Zs7qbIYMv7KybptioYFZP89ZFTMtvdGvEnRpYg70uzwSLdhZB1EPJZIrUMhybfT7Q1DYEqHwDw".to_owned());
        assert!(matches!(jwt.decode(&aud), Err(JwtError::Serialization(..))));

        // Invalid DID prefix.
        let jwt = Jwt("eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJ4ZGlkOmtleTp6Nk1rb2RIWnduZVZSU2h0YUxmOEpLWWt4cERHcDF2R1pucEdtZEJwWDhNMmV4eEgiLCJzdWIiOiJjNDc5ZmU1ZGM0NjRlNzcxZTc4YjE5M2QyMzlhNjViNThkMjc4Y2FkMWMzNGJmYjBiNTcxNmU1YmI1MTQ5MjhlIiwiYXVkIjoid3NzOi8vcmVsYXkud2FsbGV0Y29ubmVjdC5jb20iLCJpYXQiOjE2NTY5MTAwOTcsImV4cCI6NDgxMjY3MDA5N30.GGhlhz7kXCqCTUsn390O_hA9YQDa61d_DDiSVLsa70xrgFrGmjjoWWl1dsZn3RVq4V1IB0P1__NDJ2PK0OMiDA".to_owned());
        assert!(matches!(jwt.decode(&aud), Err(JwtError::Serialization(..))));

        // Invalid DID method
        let jwt = Jwt("eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJkaWQ6eGtleTp6Nk1rb2RIWnduZVZSU2h0YUxmOEpLWWt4cERHcDF2R1pucEdtZEJwWDhNMmV4eEgiLCJzdWIiOiJjNDc5ZmU1ZGM0NjRlNzcxZTc4YjE5M2QyMzlhNjViNThkMjc4Y2FkMWMzNGJmYjBiNTcxNmU1YmI1MTQ5MjhlIiwiYXVkIjoid3NzOi8vcmVsYXkud2FsbGV0Y29ubmVjdC5jb20iLCJpYXQiOjE2NTY5MTAwOTcsImV4cCI6NDgxMjY3MDA5N30.rogEwjJLQFwbDm4psUty7MPkHrCrNiXxpwEYZ2nctppmF7MYvC3g7URZNYkKxMbFtNZ1hFCwsr1peEu3pVeJCg".to_owned());
        assert!(matches!(jwt.decode(&aud), Err(JwtError::Serialization(..))));

        // Invalid issuer base58.
        let jwt = Jwt("eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJkaWQ6a2V5Ono2TWtvZEhad25lVlJTaHRhTGY4SktZa3hwREdwMXZHWm5wR21kQnBYOE0yZXh4SGwiLCJzdWIiOiJjNDc5ZmU1ZGM0NjRlNzcxZTc4YjE5M2QyMzlhNjViNThkMjc4Y2FkMWMzNGJmYjBiNTcxNmU1YmI1MTQ5MjhlIiwiYXVkIjoid3NzOi8vcmVsYXkud2FsbGV0Y29ubmVjdC5jb20iLCJpYXQiOjE2NTY5MTAwOTcsImV4cCI6NDgxMjY3MDA5N30.nLdxz4f6yJ8HsWZJUvpSHjFjoat4PfJav-kyqdHj6JXcX5SyDvp3QNB9doyzRWb9jpbA36Av0qn4kqLl-pGuBg".to_owned());
        assert!(matches!(jwt.decode(&aud), Err(JwtError::Serialization(..))));

        let keypair = SigningKey::generate(&mut OsRng);
        let sub: String = "test".to_owned();

        // IAT in future.
        let jwt = AuthToken::new(sub.clone())
            .iat(
                chrono::Utc::now()
                    + chrono::Duration::try_hours(1).expect("Safe unwrap: does not return None"),
            )
            .as_jwt(&keypair)
            .unwrap();
        assert!(matches!(
            Jwt(jwt.into()).decode(&aud),
            Err(JwtError::NotYetValid { .. })
        ));

        // IAT leeway, valid.
        let jwt = AuthToken::new(sub.clone())
            .iat(
                chrono::Utc::now()
                    + chrono::Duration::try_seconds(JWT_VALIDATION_TIME_LEEWAY_SECS)
                        .expect("Safe unwrap: does not return None"),
            )
            .as_jwt(&keypair)
            .unwrap();
        assert!(Jwt(jwt.into()).decode(&aud).is_ok());

        // IAT leeway, invalid.
        let jwt = AuthToken::new(sub.clone())
            .iat(
                chrono::Utc::now()
                    + chrono::Duration::try_seconds(JWT_VALIDATION_TIME_LEEWAY_SECS + 1)
                        .expect("Safe unwrap: does not return None"),
            )
            .as_jwt(&keypair)
            .unwrap();
        assert!(matches!(
            Jwt(jwt.into()).decode(&aud),
            Err(JwtError::NotYetValid { .. })
        ));

        // Past expiration.
        let jwt = AuthToken::new(sub.clone())
            .iat(
                chrono::Utc::now()
                    - chrono::Duration::try_hours(2).expect("Safe unwrap: does not return None"),
            )
            .ttl(Duration::from_secs(3600))
            .as_jwt(&keypair)
            .unwrap();
        assert!(matches!(
            Jwt(jwt.into()).decode(&aud),
            Err(JwtError::Expired { .. })
        ));

        // Expiration leeway, valid.
        let jwt = AuthToken::new(sub.clone())
            .iat(
                chrono::Utc::now()
                    - chrono::Duration::try_seconds(3600 + JWT_VALIDATION_TIME_LEEWAY_SECS)
                        .expect("Safe unwrap: does not return None"),
            )
            .ttl(Duration::from_secs(3600))
            .as_jwt(&keypair)
            .unwrap();
        assert!(Jwt(jwt.into()).decode(&aud).is_ok());

        // Expiration leeway, invalid.
        let jwt = AuthToken::new(sub.clone())
            .iat(
                chrono::Utc::now()
                    - chrono::Duration::try_seconds(3600 + JWT_VALIDATION_TIME_LEEWAY_SECS + 1)
                        .expect("Safe unwrap: does not return None"),
            )
            .ttl(Duration::from_secs(3600))
            .as_jwt(&keypair)
            .unwrap();
        assert!(matches!(
            Jwt(jwt.into()).decode(&aud),
            Err(JwtError::Expired { .. })
        ));

        // Invalid aud.
        let jwt = AuthToken::new(sub)
            .aud("wss://not.relay.walletconnect.com")
            .as_jwt(&keypair)
            .unwrap();
        assert!(matches!(
            Jwt(jwt.into()).decode(&aud),
            Err(JwtError::InvalidAudience)
        ));
    }
}
