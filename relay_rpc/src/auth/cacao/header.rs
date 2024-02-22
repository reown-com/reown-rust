use {
    super::CacaoError,
    serde::{Deserialize, Serialize},
    std::sync::Arc,
};

pub const EIP4361: &str = "eip4361";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub struct Header {
    pub t: Arc<str>,
}

impl Header {
    pub fn validate(&self) -> Result<(), CacaoError> {
        match self.t.as_ref() {
            EIP4361 => Ok(()),
            _ => Err(CacaoError::HeaderTypeUnsupported(self.t.clone())),
        }
    }
}
