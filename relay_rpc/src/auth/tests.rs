use {
    crate::{
        auth::{AuthToken, Jwt, JwtVerificationError, JWT_VALIDATION_TIME_LEEWAY_SECS},
        domain::{ClientIdDecodingError, DecodedAuthSubject},
    },
    ed25519_dalek::Keypair,
    std::{collections::HashSet, time::Duration},
};

#[test]
fn token_validation() {
    let aud = HashSet::from(["wss://relay.walletconnect.com".to_owned()]);

    // Invalid signature.
    let jwt = Jwt("eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJkaWQ6a2V5Ono2TWtvZEhad25lVlJTaHRhTGY4SktZa3hwREdwMXZHWm5wR21kQnBYOE0yZXh4SCIsInN1YiI6ImM0NzlmZTVkYzQ2NGU3NzFlNzhiMTkzZDIzOWE2NWI1OGQyNzhjYWQxYzM0YmZiMGI1NzE2ZTViYjUxNDkyOGUiLCJhdWQiOiJ3c3M6Ly9yZWxheS53YWxsZXRjb25uZWN0LmNvbSIsImlhdCI6MTY1NjkxMDA5NywiZXhwIjo0ODEyNjcwMDk3fQ.CLryc7bGZ_mBVh-P5p2tDDkjY8m9ji9xZXixJCbLLd4TMBh7F0EkChbWOOUQp4DyXUVK4CN-hxMZgt2xnePUBAx".to_owned());
    assert!(matches!(
        jwt.decode(&aud),
        Err(JwtVerificationError::Signature)
    ));

    // Invalid multicodec header.
    let jwt = Jwt("eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJkaWQ6a2V5Ono2TWt2eDRWVnVCQlBIekVvTERiNWdOQzRyUW1uSnN0YzFib29oS2ZjSlV0OU12NjUiLCJzdWIiOiJjNDc5ZmU1ZGM0NjRlNzcxZTc4YjE5M2QyMzlhNjViNThkMjc4Y2FkMWMzNGJmYjBiNTcxNmU1YmI1MTQ5MjhlIiwiYXVkIjoid3NzOi8vcmVsYXkud2FsbGV0Y29ubmVjdC5jb20iLCJpYXQiOjE2NTY5MTAwOTcsImV4cCI6NDgxMjY3MDA5N30.ixjxEISufsDpdsp4MRwD4Q100d8s7v4mSlIWIad6q8Nh__768pzPaCAVXQIZLxKPhuJQ92cZi7tVUJtAE1_UCg".to_owned());
    assert!(matches!(
        jwt.decode(&aud),
        Err(JwtVerificationError::PubKey(
            ClientIdDecodingError::Encoding
        ))
    ));

    // Invalid multicodec base.
    let jwt = Jwt("eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJkaWQ6a2V5Onh6Nk1rb2RIWnduZVZSU2h0YUxmOEpLWWt4cERHcDF2R1pucEdtZEJwWDhNMmV4eEgiLCJzdWIiOiJjNDc5ZmU1ZGM0NjRlNzcxZTc4YjE5M2QyMzlhNjViNThkMjc4Y2FkMWMzNGJmYjBiNTcxNmU1YmI1MTQ5MjhlIiwiYXVkIjoid3NzOi8vcmVsYXkud2FsbGV0Y29ubmVjdC5jb20iLCJpYXQiOjE2NTY5MTAwOTcsImV4cCI6NDgxMjY3MDA5N30.BINvB6JpUyp5Zs7qbIYMv7KybptioYFZP89ZFTMtvdGvEnRpYg70uzwSLdhZB1EPJZIrUMhybfT7Q1DYEqHwDw".to_owned());
    assert!(matches!(
        jwt.decode(&aud),
        Err(JwtVerificationError::PubKey(ClientIdDecodingError::Base))
    ));

    // Invalid DID prefix.
    let jwt = Jwt("eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJ4ZGlkOmtleTp6Nk1rb2RIWnduZVZSU2h0YUxmOEpLWWt4cERHcDF2R1pucEdtZEJwWDhNMmV4eEgiLCJzdWIiOiJjNDc5ZmU1ZGM0NjRlNzcxZTc4YjE5M2QyMzlhNjViNThkMjc4Y2FkMWMzNGJmYjBiNTcxNmU1YmI1MTQ5MjhlIiwiYXVkIjoid3NzOi8vcmVsYXkud2FsbGV0Y29ubmVjdC5jb20iLCJpYXQiOjE2NTY5MTAwOTcsImV4cCI6NDgxMjY3MDA5N30.GGhlhz7kXCqCTUsn390O_hA9YQDa61d_DDiSVLsa70xrgFrGmjjoWWl1dsZn3RVq4V1IB0P1__NDJ2PK0OMiDA".to_owned());
    assert!(matches!(
        jwt.decode(&aud),
        Err(JwtVerificationError::IssuerPrefix)
    ));

    // Invalid DID method
    let jwt = Jwt("eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJkaWQ6eGtleTp6Nk1rb2RIWnduZVZSU2h0YUxmOEpLWWt4cERHcDF2R1pucEdtZEJwWDhNMmV4eEgiLCJzdWIiOiJjNDc5ZmU1ZGM0NjRlNzcxZTc4YjE5M2QyMzlhNjViNThkMjc4Y2FkMWMzNGJmYjBiNTcxNmU1YmI1MTQ5MjhlIiwiYXVkIjoid3NzOi8vcmVsYXkud2FsbGV0Y29ubmVjdC5jb20iLCJpYXQiOjE2NTY5MTAwOTcsImV4cCI6NDgxMjY3MDA5N30.rogEwjJLQFwbDm4psUty7MPkHrCrNiXxpwEYZ2nctppmF7MYvC3g7URZNYkKxMbFtNZ1hFCwsr1peEu3pVeJCg".to_owned());
    assert!(matches!(
        jwt.decode(&aud),
        Err(JwtVerificationError::IssuerMethod)
    ));

    // Invalid issuer base58.
    let jwt = Jwt("eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJkaWQ6a2V5Ono2TWtvZEhad25lVlJTaHRhTGY4SktZa3hwREdwMXZHWm5wR21kQnBYOE0yZXh4SGwiLCJzdWIiOiJjNDc5ZmU1ZGM0NjRlNzcxZTc4YjE5M2QyMzlhNjViNThkMjc4Y2FkMWMzNGJmYjBiNTcxNmU1YmI1MTQ5MjhlIiwiYXVkIjoid3NzOi8vcmVsYXkud2FsbGV0Y29ubmVjdC5jb20iLCJpYXQiOjE2NTY5MTAwOTcsImV4cCI6NDgxMjY3MDA5N30.nLdxz4f6yJ8HsWZJUvpSHjFjoat4PfJav-kyqdHj6JXcX5SyDvp3QNB9doyzRWb9jpbA36Av0qn4kqLl-pGuBg".to_owned());
    assert!(matches!(
        jwt.decode(&aud),
        Err(JwtVerificationError::PubKey(
            ClientIdDecodingError::Encoding
        ))
    ));

    let keypair = Keypair::generate(&mut rand::thread_rng());

    // IAT in future.
    let jwt = AuthToken::new(DecodedAuthSubject::generate())
        .iat(chrono::Utc::now() + chrono::Duration::hours(1))
        .as_jwt(&keypair)
        .unwrap();
    assert!(matches!(
        Jwt(jwt.into()).decode(&aud),
        Err(JwtVerificationError::NotYetValid)
    ));

    // IAT leeway, valid.
    let jwt = AuthToken::new(DecodedAuthSubject::generate())
        .iat(chrono::Utc::now() + chrono::Duration::seconds(JWT_VALIDATION_TIME_LEEWAY_SECS))
        .as_jwt(&keypair)
        .unwrap();
    assert!(matches!(Jwt(jwt.into()).decode(&aud), Ok(_)));

    // IAT leeway, invalid.
    let jwt = AuthToken::new(DecodedAuthSubject::generate())
        .iat(chrono::Utc::now() + chrono::Duration::seconds(JWT_VALIDATION_TIME_LEEWAY_SECS + 1))
        .as_jwt(&keypair)
        .unwrap();
    assert!(matches!(
        Jwt(jwt.into()).decode(&aud),
        Err(JwtVerificationError::NotYetValid)
    ));

    // Past expiration.
    let jwt = AuthToken::new(DecodedAuthSubject::generate())
        .iat(chrono::Utc::now() - chrono::Duration::hours(2))
        .ttl(Duration::from_secs(3600))
        .as_jwt(&keypair)
        .unwrap();
    assert!(matches!(
        Jwt(jwt.into()).decode(&aud),
        Err(JwtVerificationError::Expired)
    ));

    // Expiration leeway, valid.
    let jwt = AuthToken::new(DecodedAuthSubject::generate())
        .iat(chrono::Utc::now() - chrono::Duration::seconds(3600 + JWT_VALIDATION_TIME_LEEWAY_SECS))
        .ttl(Duration::from_secs(3600))
        .as_jwt(&keypair)
        .unwrap();
    assert!(matches!(Jwt(jwt.into()).decode(&aud), Ok(_)));

    // Expiration leeway, invalid.
    let jwt = AuthToken::new(DecodedAuthSubject::generate())
        .iat(
            chrono::Utc::now()
                - chrono::Duration::seconds(3600 + JWT_VALIDATION_TIME_LEEWAY_SECS + 1),
        )
        .ttl(Duration::from_secs(3600))
        .as_jwt(&keypair)
        .unwrap();
    assert!(matches!(
        Jwt(jwt.into()).decode(&aud),
        Err(JwtVerificationError::Expired)
    ));

    // Invalid aud.
    let jwt = AuthToken::new(DecodedAuthSubject::generate())
        .aud("wss://not.relay.walletconnect.com")
        .as_jwt(&keypair)
        .unwrap();
    assert!(matches!(
        Jwt(jwt.into()).decode(&aud),
        Err(JwtVerificationError::InvalidAudience)
    ));
}
