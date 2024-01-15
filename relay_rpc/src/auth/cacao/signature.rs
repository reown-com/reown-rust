use {
    super::{Cacao, CacaoError},
    alloy_primitives::Address,
    alloy_providers::provider::TempProvider,
    alloy_rpc_types::{CallInput, CallRequest},
    alloy_sol_types::{sol, SolCall},
    serde::{Deserialize, Serialize},
    sha3::{Digest, Keccak256},
    std::str::FromStr,
    url::Url,
};

pub const EIP191: &str = "eip191";
pub const EIP1271: &str = "eip1271";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub struct Signature {
    pub t: String,
    pub s: String,
}

impl Signature {
    pub async fn verify(&self, cacao: &Cacao, provider: Option<Url>) -> Result<bool, CacaoError> {
        let address = cacao.p.address()?;

        let signature = data_encoding::HEXLOWER_PERMISSIVE
            .decode(strip_hex_prefix(&cacao.s.s).as_bytes())
            .map_err(|_| CacaoError::Verification)?;

        let hash = Keccak256::new_with_prefix(eip191_bytes(&cacao.siwe_message()?));

        match self.t.as_str() {
            EIP191 => Eip191.verify(&signature, &address, hash),
            EIP1271 if provider.is_some() => {
                Eip1271
                    .verify(
                        signature,
                        Address::from_str(&address).map_err(|_| CacaoError::AddressInvalid)?,
                        &hash.finalize()[..]
                            .try_into()
                            .expect("hash length is 32 bytes"),
                        provider.expect("provider is some"),
                    )
                    .await
            }
            _ => Err(CacaoError::UnsupportedSignature),
        }
    }
}

pub fn eip191_bytes(message: &str) -> Vec<u8> {
    format!(
        "\u{0019}Ethereum Signed Message:\n{}{}",
        message.as_bytes().len(),
        message
    )
    .into()
}

pub struct Eip191;

impl Eip191 {
    fn verify(&self, signature: &[u8], address: &str, hash: Keccak256) -> Result<bool, CacaoError> {
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
}

/// Remove the "0x" prefix from a hex string.
fn strip_hex_prefix(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}

pub struct Eip1271;

// https://eips.ethereum.org/EIPS/eip-1271
const MAGIC_VALUE: u32 = 0x1626ba7e;
sol! {
    function isValidSignature(
        bytes32 _hash,
        bytes memory _signature)
        public
        view
        returns (bytes4 magicValue);
}

impl Eip1271 {
    async fn verify(
        &self,
        signature: Vec<u8>,
        address: Address,
        hash: &[u8; 32],
        provider: Url,
    ) -> Result<bool, CacaoError> {
        let provider =
            alloy_providers::provider::Provider::new(alloy_transport_http::Http::new(provider));

        let call_request = CallRequest {
            to: Some(address),
            input: CallInput::new(
                isValidSignatureCall {
                    _hash: alloy_primitives::FixedBytes::from(hash),
                    _signature: signature,
                }
                .abi_encode()
                .into(),
            ),
            ..Default::default()
        };

        let result = provider.call(call_request, None).await.map_err(|e| {
            if let Some(error_response) = e.as_error_resp() {
                if error_response.message.starts_with("execution reverted:") {
                    CacaoError::Verification
                } else {
                    CacaoError::Eip1271Internal(e)
                }
            } else {
                CacaoError::Eip1271Internal(e)
            }
        })?;

        if result[..4] == MAGIC_VALUE.to_be_bytes().to_vec() {
            Ok(true)
        } else {
            Err(CacaoError::Verification)
        }
    }
}
