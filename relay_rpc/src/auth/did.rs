pub const DID_DELIMITER: &str = ":";
pub const DID_PREFIX: &str = "did";
pub const DID_METHOD_KEY: &str = "key";
pub const DID_METHOD_PKH: &str = "pkh";

#[derive(Debug, Clone, thiserror::Error)]
pub enum DidError {
    #[error("Invalid issuer DID prefix")]
    Prefix,

    #[error("Invalid issuer DID method")]
    Method,

    #[error("Invalid issuer format")]
    Format,
}

pub fn extract_did_data<'a>(did: &'a str, method: &str) -> Result<&'a str, DidError> {
    did.strip_prefix(DID_PREFIX)
        .ok_or(DidError::Prefix)?
        .strip_prefix(DID_DELIMITER)
        .ok_or(DidError::Format)?
        .strip_prefix(method)
        .ok_or(DidError::Method)?
        .strip_prefix(DID_DELIMITER)
        .ok_or(DidError::Format)
}

pub fn combine_did_data(method: &str, data: &str) -> String {
    format!("{DID_PREFIX}{DID_DELIMITER}{method}{DID_DELIMITER}{data}")
}
