use {
    super::CacaoError,
    alloy_primitives::{Address, FixedBytes},
    alloy_providers::provider::{Provider, TempProvider},
    alloy_rpc_types::{CallInput, CallRequest},
    alloy_sol_types::{sol, SolCall},
    alloy_transport_http::Http,
    url::Url,
};

pub mod blockchain_api;
pub mod get_rpc_url;

pub const EIP1271: &str = "eip1271";

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

pub async fn verify_eip1271(
    signature: Vec<u8>,
    address: Address,
    hash: &[u8; 32],
    provider: Url,
) -> Result<bool, CacaoError> {
    let provider = Provider::new(Http::new(provider));

    let call_request = CallRequest {
        to: Some(address),
        input: CallInput::new(
            isValidSignatureCall {
                _hash: FixedBytes::from(hash),
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

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::auth::cacao::signature::{eip191::eip191_bytes, strip_hex_prefix},
        alloy_primitives::address,
        sha3::{Digest, Keccak256},
    };

    // Manual test. Paste address, signature, message, and project ID to verify
    // function
    #[tokio::test]
    #[ignore]
    async fn test_eip1271() {
        let address = address!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        let signature = "xxx";
        let signature = data_encoding::HEXLOWER_PERMISSIVE
            .decode(strip_hex_prefix(signature).as_bytes())
            .map_err(|_| CacaoError::Verification)
            .unwrap();
        let message = "xxx";
        let hash = &Keccak256::new_with_prefix(eip191_bytes(message)).finalize()[..]
            .try_into()
            .unwrap();
        let provider = "https://rpc.walletconnect.com/v1?chainId=eip155:1&projectId=xxx"
            .parse()
            .unwrap();
        assert!(verify_eip1271(signature, address, hash, provider)
            .await
            .unwrap());
    }
}
