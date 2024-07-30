use {
    crate::{
        domain::{MessageId, Topic},
        jwt::{JwtBasicClaims, VerifyableClaims},
    },
    serde::{Deserialize, Serialize},
    std::sync::Arc,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatchType {
    Subscriber,
    Publisher,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatchStatus {
    Accepted,
    Queued,
    Delivered,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WatchAction {
    #[serde(rename = "irn_watchRegister")]
    Register,
    #[serde(rename = "irn_watchUnregister")]
    Unregister,
    #[serde(rename = "irn_watchEvent")]
    WatchEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WatchRegisterClaims {
    /// Basic JWT claims.
    #[serde(flatten)]
    pub basic: JwtBasicClaims,
    /// Action. Must be `irn_watchRegister`.
    pub act: WatchAction,
    /// Watcher type. Either subscriber or publisher.
    pub typ: WatchType,
    /// Webhook URL.
    pub whu: String,
    /// Array of message tags to watch.
    pub tag: Vec<u32>,
    /// Array of statuses to watch.
    pub sts: Vec<WatchStatus>,
}

impl VerifyableClaims for WatchRegisterClaims {
    fn basic(&self) -> &JwtBasicClaims {
        &self.basic
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WatchUnregisterClaims {
    /// Basic JWT claims.
    #[serde(flatten)]
    pub basic: JwtBasicClaims,
    /// Action. Must be `irn_watchUnregister`.
    pub act: WatchAction,
    /// Watcher type. Either subscriber or publisher.
    pub typ: WatchType,
    /// Webhook URL.
    pub whu: String,
}

impl VerifyableClaims for WatchUnregisterClaims {
    fn basic(&self) -> &JwtBasicClaims {
        &self.basic
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchEventPayload {
    /// Message ID.
    pub message_id: MessageId,
    /// Webhook status. Either `accepted`, `queued` or `delivered`.
    pub status: WatchStatus,
    /// Topic of the message that triggered the watch event.
    pub topic: Topic,
    /// The published message.
    pub message: Arc<str>,
    /// The Verify attestation JWT.
    pub attestation: Option<Arc<str>>,
    /// Message publishing timestamp.
    pub published_at: i64,
    /// Message tag.
    pub tag: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WatchEventClaims {
    /// Basic JWT claims.
    #[serde(flatten)]
    pub basic: JwtBasicClaims,
    /// Action. Must be `irn_watchEvent`.
    pub act: WatchAction,
    /// Watcher type. Either subscriber or publisher.
    pub typ: WatchType,
    /// Webhook URL.
    pub whu: String,
    /// Event payload.
    pub evt: WatchEventPayload,
}

impl VerifyableClaims for WatchEventClaims {
    fn basic(&self) -> &JwtBasicClaims {
        &self.basic
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchWebhookPayload {
    /// JWT with [`WatchEventClaims`] payload.
    pub event_auth: Vec<String>,
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{auth::RELAY_WEBSOCKET_ADDRESS, domain::DecodedClientId},
        chrono::DateTime,
        ed25519_dalek::SigningKey,
    };

    const KEYPAIR: [u8; 32] = [
        215, 142, 127, 216, 153, 183, 205, 110, 103, 118, 181, 195, 60, 71, 5, 221, 100, 196, 207,
        81, 229, 11, 116, 121, 235, 104, 1, 121, 25, 18, 218, 83,
    ];

    #[test]
    fn watch_register_jwt() {
        let key = SigningKey::from_bytes(&KEYPAIR);
        let iat = DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z").unwrap();
        let exp = DateTime::parse_from_rfc3339("3000-01-01T00:00:00Z").unwrap();

        let claims = WatchRegisterClaims {
            basic: JwtBasicClaims {
                iss: DecodedClientId::from_key(&key.verifying_key()).into(),
                aud: RELAY_WEBSOCKET_ADDRESS.to_owned(),
                sub: "https://example.com".to_owned(),
                iat: iat.timestamp(),
                exp: Some(exp.timestamp()),
            },
            act: WatchAction::Register,
            typ: WatchType::Subscriber,
            whu: "https://example.com".to_owned(),
            tag: vec![1100],
            sts: vec![WatchStatus::Accepted],
        };

        // Verify that the fields are flattened, and that enums are serialized in
        // lowercase.
        assert_eq!(
            serde_json::to_string(&claims).unwrap(),
            r#"{"iss":"did:key:z6Mku3wsRZTAHjr6xrYWVUfyGeNSNz1GJRVfazp3N76AL9gE","aud":"wss://relay.walletconnect.com","sub":"https://example.com","iat":946684800,"exp":32503680000,"act":"irn_watchRegister","typ":"subscriber","whu":"https://example.com","tag":[1100],"sts":["accepted"]}"#
        );

        // Verify that the claims can be encoded and decoded correctly.
        assert_eq!(
            claims,
            WatchRegisterClaims::try_from_str(&claims.encode(&key).unwrap()).unwrap()
        );
    }

    #[test]
    fn watch_unregister_jwt() {
        let key = SigningKey::from_bytes(&KEYPAIR);
        let iat = DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z").unwrap();
        let exp = DateTime::parse_from_rfc3339("3000-01-01T00:00:00Z").unwrap();

        let claims = WatchUnregisterClaims {
            basic: JwtBasicClaims {
                iss: DecodedClientId::from_key(&key.verifying_key()).into(),
                aud: RELAY_WEBSOCKET_ADDRESS.to_owned(),
                sub: "https://example.com".to_owned(),
                iat: iat.timestamp(),
                exp: Some(exp.timestamp()),
            },
            act: WatchAction::Unregister,
            typ: WatchType::Publisher,
            whu: "https://example.com".to_owned(),
        };

        // Verify that the fields are flattened, and that enums are serialized in
        // lowercase.
        assert_eq!(
            serde_json::to_string(&claims).unwrap(),
            r#"{"iss":"did:key:z6Mku3wsRZTAHjr6xrYWVUfyGeNSNz1GJRVfazp3N76AL9gE","aud":"wss://relay.walletconnect.com","sub":"https://example.com","iat":946684800,"exp":32503680000,"act":"irn_watchUnregister","typ":"publisher","whu":"https://example.com"}"#
        );

        // Verify that the claims can be encoded and decoded correctly.
        assert_eq!(
            claims,
            WatchUnregisterClaims::try_from_str(&claims.encode(&key).unwrap()).unwrap()
        );
    }

    #[test]
    fn watch_event_jwt() {
        let key = SigningKey::from_bytes(&KEYPAIR);
        let iat = DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z").unwrap();
        let exp = DateTime::parse_from_rfc3339("3000-01-01T00:00:00Z").unwrap();
        let topic = Topic::from("474e88153f4db893de42c35e1891dc0e37a02e11961385de0475460fb48b8639");

        let claims = WatchEventClaims {
            basic: JwtBasicClaims {
                iss: DecodedClientId::from_key(&key.verifying_key()).into(),
                aud: RELAY_WEBSOCKET_ADDRESS.to_owned(),
                sub: "https://example.com".to_owned(),
                iat: iat.timestamp(),
                exp: Some(exp.timestamp()),
            },
            act: WatchAction::WatchEvent,
            whu: "https://example.com".to_owned(),
            typ: WatchType::Subscriber,
            evt: WatchEventPayload {
                message_id: 12345678.into(),
                status: WatchStatus::Accepted,
                topic,
                message: Arc::from("test message"),
                attestation: Some(Arc::from("test attestation")),
                published_at: iat.timestamp(),
                tag: 1100,
            },
        };

        // Verify that the fields are flattened, and that enums are serialized in
        // lowercase.
        assert_eq!(
            serde_json::to_string(&claims).unwrap(),
            r#"{"iss":"did:key:z6Mku3wsRZTAHjr6xrYWVUfyGeNSNz1GJRVfazp3N76AL9gE","aud":"wss://relay.walletconnect.com","sub":"https://example.com","iat":946684800,"exp":32503680000,"act":"irn_watchEvent","typ":"subscriber","whu":"https://example.com","evt":{"messageId":12345678,"status":"accepted","topic":"474e88153f4db893de42c35e1891dc0e37a02e11961385de0475460fb48b8639","message":"test message","attestation":"test attestation","publishedAt":946684800,"tag":1100}}"#
        );

        // Verify that the claims can be encoded and decoded correctly.
        assert_eq!(
            claims,
            WatchEventClaims::try_from_str(&claims.encode(&key).unwrap()).unwrap()
        );
    }
}
