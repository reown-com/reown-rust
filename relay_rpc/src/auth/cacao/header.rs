use {
    super::CacaoError,
    serde::{Deserialize, Serialize},
};

pub const EIP4361: &str = "eip4361";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub struct Header {
    pub t: String,
}

impl Header {
    pub fn validate(&self) -> Result<(), CacaoError> {
        match self.t.as_str() {
            EIP4361 => Ok(()),
            _ => Err(CacaoError::Header),
        }
    }
}
