use {
    super::{CacaoError, Version},
    crate::auth::did::{extract_did_data, DID_METHOD_KEY},
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
    url::Url,
};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub struct Payload {
    pub domain: String,
    pub iss: String,
    pub statement: Option<String>,
    pub aud: String,
    pub version: Version,
    pub nonce: String,
    pub iat: String,
    pub exp: Option<String>,
    pub nbf: Option<String>,
    pub request_id: Option<String>,
    pub resources: Option<Vec<String>>,
}

impl Payload {
    const ISS_DELIMITER: &'static str = ":";
    const ISS_POSITION_OF_ADDRESS: usize = 4;
    const ISS_POSITION_OF_NAMESPACE: usize = 2;
    const ISS_POSITION_OF_REFERENCE: usize = 3;
    pub const WALLETCONNECT_IDENTITY_KEY: &'static str = "walletconnect_identity_key";

    pub fn validate(&self) -> Result<(), CacaoError> {
        self.validate_at(Utc::now())
    }

    pub fn validate_at(&self, now: DateTime<Utc>) -> Result<(), CacaoError> {
        if let Some(exp) = &self.exp {
            let exp_time = parse_cacao_timestamp(exp)?;
            if now >= exp_time {
                return Err(CacaoError::Expired);
            }
        }
        if let Some(nbf) = &self.nbf {
            let nbf_time = parse_cacao_timestamp(nbf)?;
            if now < nbf_time {
                return Err(CacaoError::NotYetValid);
            }
        }
        Ok(())
    }

    pub fn address(&self) -> Result<String, CacaoError> {
        self.iss
            .split(Self::ISS_DELIMITER)
            .nth(Self::ISS_POSITION_OF_ADDRESS)
            .ok_or(CacaoError::PayloadResources)
            .map(|s| s.to_string())
    }

    pub fn namespace(&self) -> Result<String, CacaoError> {
        self.iss
            .split(Self::ISS_DELIMITER)
            .nth(Self::ISS_POSITION_OF_NAMESPACE)
            .ok_or(CacaoError::PayloadResources)
            .map(|s| s.to_string())
    }

    pub fn chain_id_reference(&self) -> Result<String, CacaoError> {
        Ok(format!(
            "{}{}{}",
            self.namespace()?,
            Self::ISS_DELIMITER,
            self.chain_id()?
        ))
    }

    pub fn chain_id(&self) -> Result<String, CacaoError> {
        self.iss
            .split(Self::ISS_DELIMITER)
            .nth(Self::ISS_POSITION_OF_REFERENCE)
            .ok_or(CacaoError::PayloadResources)
            .map(|s| s.to_string())
    }

    pub fn caip_10_address(&self) -> Result<String, CacaoError> {
        Ok(format!(
            "{}{}{}",
            self.chain_id_reference()?,
            Self::ISS_DELIMITER,
            self.address()?
        ))
    }

    pub fn identity_key(&self) -> Result<String, CacaoError> {
        self.identity_key_from_audience()
            .or_else(|_| self.identity_key_from_resources())
    }

    fn extract_did_key(did_key: &str) -> Result<String, CacaoError> {
        extract_did_data(did_key, DID_METHOD_KEY)
            .map_err(|_| CacaoError::PayloadIdentityKey)
            .map(|data| data.to_owned())
    }

    fn identity_key_from_resources(&self) -> Result<String, CacaoError> {
        let resources = self
            .resources
            .as_ref()
            .ok_or(CacaoError::PayloadResources)?;
        let did_key = resources.first().ok_or(CacaoError::PayloadIdentityKey)?;

        Self::extract_did_key(did_key)
    }

    fn identity_key_from_audience(&self) -> Result<String, CacaoError> {
        self.identity_key_from_audience_url()
            .or_else(|_| self.identity_key_from_audience_did_key())
    }

    fn identity_key_from_audience_did_key(&self) -> Result<String, CacaoError> {
        Self::extract_did_key(&self.aud)
    }

    fn identity_key_from_audience_url(&self) -> Result<String, CacaoError> {
        self.aud
            .parse::<Url>()
            .map_err(|_| CacaoError::PayloadIdentityKey)
            .and_then(|url| {
                url.query_pairs()
                    .find(|(key, _)| key == Self::WALLETCONNECT_IDENTITY_KEY)
                    .ok_or(CacaoError::PayloadIdentityKey)
                    .and_then(|(_, value)| Self::extract_did_key(&value))
            })
    }
}

fn parse_cacao_timestamp(value: &str) -> Result<DateTime<Utc>, CacaoError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| CacaoError::TimestampInvalid)
}

#[cfg(all(test, feature = "cacao-tests"))]
mod tests {
    use super::*;

    #[test]
    fn identity_key_from_resources() {
        assert_eq!(
            Payload {
                domain: "example.com".to_owned(),
                iss: "did:pkh:eip155:1:0xdFe7d0E324ed017a74aE311E9236E6CaDB24176b".to_owned(),
                statement: None,
                aud: "".to_owned(),
                version: Version::V1,
                nonce: "".to_owned(),
                iat: "2023-09-07T11:04:23+02:00".to_owned(),
                exp: None,
                nbf: None,
                request_id: None,
                resources: Some(vec![
                    "did:key:z6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht7".to_owned(),
                ]),
            }
            .identity_key()
            .unwrap(),
            "z6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht7"
        );
    }

    #[test]
    fn identity_key_from_aud() {
        assert_eq!(
            Payload {
                domain: "example.com".to_owned(),
                iss: "did:pkh:eip155:1:0xdFe7d0E324ed017a74aE311E9236E6CaDB24176b".to_owned(),
                statement: None,
                aud: "did:key:z6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht7".to_owned(),
                version: Version::V1,
                nonce: "".to_owned(),
                iat: "2023-09-07T11:04:23+02:00".to_owned(),
                exp: None,
                nbf: None,
                request_id: None,
                resources: Some(vec![
                    "did:key:z6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht8".to_owned(),
                ]),
            }
            .identity_key()
            .unwrap(),
            "z6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht7"
        );
    }

    #[test]
    fn identity_key_from_aud_url() {
        assert_eq!(
            Payload {
                domain: "example.com".to_owned(),
                iss: "did:pkh:eip155:1:0xdFe7d0E324ed017a74aE311E9236E6CaDB24176b".to_owned(),
                statement: None,
                aud: "https://example.com?walletconnect_identity_key=did:key:z6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht7".to_owned(),
                version: Version::V1,
                nonce: "".to_owned(),
                iat: "2023-09-07T11:04:23+02:00".to_owned(),
                exp: None,
                nbf: None,
                request_id: None,
                resources: Some(vec![
                    "did:key:z6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht8".to_owned(),
                ]),
            }
            .identity_key()
            .unwrap(),
            "z6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht7"
        );
    }

    #[test]
    fn identity_key_from_aud_url_encoded() {
        assert_eq!(
            Payload {
                domain: "example.com".to_owned(),
                iss: "did:pkh:eip155:1:0xdFe7d0E324ed017a74aE311E9236E6CaDB24176b".to_owned(),
                statement: None,
                aud: "https://example.com?walletconnect_identity_key=did%3Akey%3Az6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht7".to_owned(),
                version: Version::V1,
                nonce: "".to_owned(),
                iat: "2023-09-07T11:04:23+02:00".to_owned(),
                exp: None,
                nbf: None,
                request_id: None,
                resources: Some(vec![
                    "did:key:z6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht8".to_owned(),
                ]),
            }
            .identity_key()
            .unwrap(),
            "z6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht7"
        );
    }
}

#[cfg(test)]
mod validity_window_tests {
    use {super::*, chrono::TimeZone};

    fn payload(exp: Option<&str>, nbf: Option<&str>) -> Payload {
        Payload {
            domain: "example.com".to_owned(),
            iss: "did:pkh:eip155:1:0xdFe7d0E324ed017a74aE311E9236E6CaDB24176b".to_owned(),
            statement: None,
            aud: "https://example.com".to_owned(),
            version: Version::V1,
            nonce: "nonce".to_owned(),
            iat: "2023-09-07T11:04:23+02:00".to_owned(),
            exp: exp.map(str::to_owned),
            nbf: nbf.map(str::to_owned),
            request_id: None,
            resources: None,
        }
    }

    fn now() -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn absent_exp_and_nbf_are_valid() {
        assert!(payload(None, None).validate_at(now()).is_ok());
    }

    #[test]
    fn future_exp_is_valid() {
        assert!(payload(Some("2024-01-01T00:00:00Z"), None)
            .validate_at(now())
            .is_ok());
    }

    #[test]
    fn past_exp_is_expired() {
        assert!(matches!(
            payload(Some("2023-01-01T00:00:00Z"), None).validate_at(now()),
            Err(CacaoError::Expired)
        ));
    }

    #[test]
    fn exp_equal_to_now_is_expired() {
        assert!(matches!(
            payload(Some("2023-11-14T22:13:20Z"), None).validate_at(now()),
            Err(CacaoError::Expired)
        ));
    }

    #[test]
    fn past_nbf_is_valid() {
        assert!(payload(None, Some("2023-01-01T00:00:00Z"))
            .validate_at(now())
            .is_ok());
    }

    #[test]
    fn future_nbf_is_not_yet_valid() {
        assert!(matches!(
            payload(None, Some("2024-01-01T00:00:00Z")).validate_at(now()),
            Err(CacaoError::NotYetValid)
        ));
    }

    #[test]
    fn unparseable_exp_fails_closed() {
        assert!(matches!(
            payload(Some("not-a-timestamp"), None).validate_at(now()),
            Err(CacaoError::TimestampInvalid)
        ));
    }

    #[test]
    fn unparseable_nbf_fails_closed() {
        assert!(matches!(
            payload(None, Some("not-a-timestamp")).validate_at(now()),
            Err(CacaoError::TimestampInvalid)
        ));
    }
}
