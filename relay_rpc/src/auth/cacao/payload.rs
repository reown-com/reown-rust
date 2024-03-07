use {
    super::{CacaoError, Version},
    crate::auth::did::{extract_did_data, DID_METHOD_KEY},
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
    pub const WALLETCONNECT_IDENTITY_TOKEN: &'static str = "walletconnect_identity_token";

    /// TODO: write valdation
    pub fn validate(&self) -> Result<(), CacaoError> {
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
                    .find(|(key, _)| key == Self::WALLETCONNECT_IDENTITY_TOKEN)
                    .ok_or(CacaoError::PayloadIdentityKey)
                    .and_then(|(_, value)| Self::extract_did_key(&value))
            })
    }
}

#[cfg(test)]
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
                aud: "https://example.com?walletconnect_identity_token=did:key:z6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht7".to_owned(),
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
                aud: "https://example.com?walletconnect_identity_token=did%3Akey%3Az6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht7".to_owned(),
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
