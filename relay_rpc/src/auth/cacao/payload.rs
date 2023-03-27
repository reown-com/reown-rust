use {
    super::{CacaoError, Version},
    crate::auth::did::{extract_did_data, DID_METHOD_KEY},
    serde::{Deserialize, Serialize},
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
        )
        .to_lowercase())
    }

    pub fn identity_key(&self) -> Result<String, CacaoError> {
        let resources = self
            .resources
            .as_ref()
            .ok_or(CacaoError::PayloadResources)?;
        let did_key = resources.first().ok_or(CacaoError::PayloadIdentityKey)?;

        extract_did_data(did_key, DID_METHOD_KEY)
            .map(|data| data.to_string())
            .map_err(|_| CacaoError::PayloadIdentityKey)
    }
}
