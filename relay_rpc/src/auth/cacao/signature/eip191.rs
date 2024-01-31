use {
    super::CacaoError,
    crate::auth::cacao::signature::strip_hex_prefix,
    sha3::{Digest, Keccak256},
};

pub const EIP191: &str = "eip191";

pub fn eip191_bytes(message: &str) -> Vec<u8> {
    format!(
        "\u{0019}Ethereum Signed Message:\n{}{}",
        message.as_bytes().len(),
        message
    )
    .into()
}

pub fn verify_eip191(signature: &[u8], address: &str, hash: Keccak256) -> Result<bool, CacaoError> {
    use k256::ecdsa::{RecoveryId, Signature as Sig, VerifyingKey};

    let sig = Sig::try_from(&signature[..64]).map_err(|_| CacaoError::Verification)?;
    let recovery_id =
        RecoveryId::try_from(&signature[64] % 27).map_err(|_| CacaoError::Verification)?;

    let recovered_key = VerifyingKey::recover_from_digest(hash, &sig, recovery_id)
        .map_err(|_| CacaoError::Verification)?;

    let add = &Keccak256::default()
        .chain_update(&recovered_key.to_encoded_point(false).as_bytes()[1..])
        .finalize()[12..];

    let address_encoded = data_encoding::HEXLOWER_PERMISSIVE.encode(add);

    if address_encoded.to_lowercase() != strip_hex_prefix(address).to_lowercase() {
        Err(CacaoError::Verification)
    } else {
        Ok(true)
    }
}
