use {
    super::{Cacao, CacaoError},
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub struct Signature {
    pub t: String,
    pub s: String,
}

impl Signature {
    pub fn verify(&self, cacao: &Cacao) -> Result<bool, CacaoError> {
        match self.t.as_str() {
            "eip191" => Eip191.verify(&cacao.s.s, &cacao.p.address()?, &cacao.siwe_message()?),
            // "eip1271" => Eip1271.verify(), TODO: How to accces our RPC?
            _ => Err(CacaoError::UnsupportedSignature),
        }
    }
}

pub struct Eip191;

impl Eip191 {
    pub fn eip191_bytes(&self, message: &str) -> Vec<u8> {
        format!(
            "\u{0019}Ethereum Signed Message:\n{}{}",
            message.as_bytes().len(),
            message
        )
        .into()
    }

    fn verify(&self, signature: &str, address: &str, message: &str) -> Result<bool, CacaoError> {
        use {
            k256::ecdsa::{RecoveryId, Signature as Sig, VerifyingKey},
            sha3::{Digest, Keccak256},
        };

        let signature_bytes = data_encoding::HEXLOWER_PERMISSIVE
            .decode(strip_hex_prefix(signature).as_bytes())
            .map_err(|_| CacaoError::Verification)?;

        let sig = Sig::try_from(&signature_bytes[..64]).map_err(|_| CacaoError::Verification)?;
        let recovery_id = RecoveryId::try_from(&signature_bytes[64] % 27)
            .map_err(|_| CacaoError::Verification)?;

        let recovered_key = VerifyingKey::recover_from_digest(
            Keccak256::new_with_prefix(self.eip191_bytes(message)),
            &sig,
            recovery_id,
        )
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
}

/// Remove the "0x" prefix from a hex string.
fn strip_hex_prefix(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}
