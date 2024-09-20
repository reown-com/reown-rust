use {
    super::CacaoError,
    crate::auth::cacao::signature::strip_hex_prefix,
    alloy::primitives::Address,
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

pub fn verify_eip191(
    signature: &[u8],
    address: &Address,
    hash: Keccak256,
) -> Result<(), CacaoError> {
    use k256::ecdsa::{RecoveryId, Signature as Sig, VerifyingKey};

    let sig = Sig::try_from(signature.get(..64).ok_or(CacaoError::Verification)?)
        .map_err(|_| CacaoError::Verification)?;
    let recovery_id = RecoveryId::try_from(signature.get(64).ok_or(CacaoError::Verification)? % 27)
        .map_err(|_| CacaoError::Verification)?;

    let recovered_key = VerifyingKey::recover_from_digest(hash, &sig, recovery_id)
        .map_err(|_| CacaoError::Verification)?;

    let hash = Keccak256::default()
        .chain_update(
            recovered_key
                .to_encoded_point(false)
                .as_bytes()
                .get(1..)
                .ok_or(CacaoError::Verification)?,
        )
        .finalize();
    let add = hash.get(12..).ok_or(CacaoError::Verification)?;

    let address_encoded = data_encoding::HEXLOWER_PERMISSIVE.encode(add);

    if address_encoded.to_lowercase() != strip_hex_prefix(&address.to_string()).to_lowercase() {
        Err(CacaoError::Verification)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::auth::cacao::signature::{
            eip191::verify_eip191,
            test_helpers::{message_hash_internal, sign_message},
        },
        alloy::primitives::Address,
        k256::ecdsa::SigningKey,
    };

    #[test]
    fn test_eip191() {
        let private_key = SigningKey::random(&mut rand::thread_rng());
        let message = "xxx";
        let signature = sign_message(message, &private_key);
        let address = Address::from_private_key(&private_key);
        verify_eip191(&signature, &address, message_hash_internal(message)).unwrap();
    }

    #[test]
    fn test_eip191_wrong_signature() {
        let private_key = SigningKey::random(&mut rand::thread_rng());
        let message = "xxx";
        let mut signature = sign_message(message, &private_key);
        *signature.first_mut().unwrap() = signature.first().unwrap().wrapping_add(1);
        let address = Address::from_private_key(&private_key);
        assert!(verify_eip191(&signature, &address, message_hash_internal(message)).is_err());
    }

    #[test]
    fn test_eip191_wrong_address() {
        let private_key = SigningKey::random(&mut rand::thread_rng());
        let message = "xxx";
        let signature = sign_message(message, &private_key);
        let mut address = Address::from_private_key(&private_key);
        *address.0.first_mut().unwrap() = address.0.first().unwrap().wrapping_add(1);
        assert!(verify_eip191(&signature, &address, message_hash_internal(message)).is_err());
    }

    #[test]
    fn test_eip191_wrong_message() {
        let private_key = SigningKey::random(&mut rand::thread_rng());
        let message = "xxx";
        let signature = sign_message(message, &private_key);
        let address = Address::from_private_key(&private_key);
        let message2 = "yyy";
        assert!(verify_eip191(&signature, &address, message_hash_internal(message2)).is_err());
    }
}
