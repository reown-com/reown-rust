use {
    super::CacaoError,
    alloy_primitives::{Address, Signature},
};

pub const EIP191: &str = "eip191";

pub fn verify_eip191(
    signature: &[u8],
    address: &Address,
    message: &[u8],
) -> Result<(), CacaoError> {
    let signature = Signature::try_from(signature).map_err(|_| CacaoError::Verification)?;
    let add = signature
        .recover_address_from_msg(message)
        .map_err(|_| CacaoError::Verification)?;

    if &add == address {
        Ok(())
    } else {
        Err(CacaoError::Verification)
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::auth::cacao::signature::eip191::verify_eip191,
        alloy_primitives::{eip191_hash_message, Address},
        k256::ecdsa::SigningKey,
    };

    pub fn sign_message(message: &str, private_key: &SigningKey) -> Vec<u8> {
        let (signature, recovery): (k256::ecdsa::Signature, _) = private_key
            .sign_prehash_recoverable(eip191_hash_message(message).as_slice())
            .unwrap();
        let signature = signature.to_bytes();
        // need for +27 is mentioned in EIP-1271 reference implementation
        [&signature[..], &[recovery.to_byte() + 27]].concat()
    }

    #[test]
    fn test_eip191() {
        let private_key = SigningKey::random(&mut rand::thread_rng());
        let message = "xxx";
        let signature = sign_message(message, &private_key);
        let address = Address::from_private_key(&private_key);
        verify_eip191(&signature, &address, message.as_bytes()).unwrap();
    }

    #[test]
    fn test_eip191_wrong_signature() {
        let private_key = SigningKey::random(&mut rand::thread_rng());
        let message = "xxx";
        let mut signature = sign_message(message, &private_key);
        *signature.first_mut().unwrap() = signature.first().unwrap().wrapping_add(1);
        let address = Address::from_private_key(&private_key);
        assert!(verify_eip191(&signature, &address, message.as_bytes()).is_err());
    }

    #[test]
    fn test_eip191_wrong_address() {
        let private_key = SigningKey::random(&mut rand::thread_rng());
        let message = "xxx";
        let signature = sign_message(message, &private_key);
        let mut address = Address::from_private_key(&private_key);
        *address.0.first_mut().unwrap() = address.0.first().unwrap().wrapping_add(1);
        assert!(verify_eip191(&signature, &address, message.as_bytes()).is_err());
    }

    #[test]
    fn test_eip191_wrong_message() {
        let private_key = SigningKey::random(&mut rand::thread_rng());
        let message = "xxx";
        let signature = sign_message(message, &private_key);
        let address = Address::from_private_key(&private_key);
        let message2 = "yyy";
        assert!(verify_eip191(&signature, &address, message2.as_bytes()).is_err());
    }
}
