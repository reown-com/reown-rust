use {
    super::CacaoError,
    alloy_primitives::{Address, FixedBytes},
    alloy_providers::provider::{Provider, TempProvider},
    alloy_rpc_types::{CallInput, CallRequest},
    alloy_sol_types::{sol, SolCall},
    alloy_transport_http::Http,
    url::Url,
};

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

    let magic = result.get(..4);
    if let Some(magic) = magic {
        if magic == MAGIC_VALUE.to_be_bytes().to_vec() {
            Ok(true)
        } else {
            Err(CacaoError::Verification)
        }
    } else {
        Err(CacaoError::Verification)
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::auth::cacao::signature::{eip191::eip191_bytes, strip_hex_prefix},
        alloy_node_bindings::{Anvil, AnvilInstance},
        alloy_primitives::address,
        k256::{ecdsa::SigningKey, elliptic_curve::SecretKey, Secp256k1},
        regex::Regex,
        sha3::{Digest, Keccak256},
        std::process::Stdio,
        tokio::process::Command,
    };

    // Manual test. Paste address, signature, message, and project ID to verify
    // function
    #[tokio::test]
    #[ignore]
    async fn test_eip1271_manual() {
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

    async fn spawn_anvil() -> (AnvilInstance, Url, SecretKey<Secp256k1>) {
        let anvil = Anvil::new().spawn();
        let provider = anvil.endpoint().parse().unwrap();
        let private_key = anvil.keys().first().unwrap().clone();
        (anvil, provider, private_key)
    }

    async fn deploy_contract(rpc_url: &Url, private_key: &SecretKey<Secp256k1>) -> Address {
        let key_encoded = data_encoding::HEXLOWER_PERMISSIVE.encode(&private_key.to_bytes());
        let output = Command::new("forge")
            .args([
                "create",
                "--contracts",
                "relay_rpc/src/auth/cacao/signature/eip1271",
                "TestContract",
                "--rpc-url",
                rpc_url.as_str(),
                "--private-key",
                &key_encoded,
                "--cache-path",
                "target/.forge/cache",
                "--out",
                "target/.forge/out",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
            .wait_with_output()
            .await
            .unwrap();
        let output = String::from_utf8(output.stdout).unwrap();
        let (_, [contract_address]) = Regex::new("Deployed to: (0x[0-9a-fA-F]+)")
            .unwrap()
            .captures(&output)
            .unwrap()
            .extract();
        contract_address.parse().unwrap()
    }

    fn sign_message(message: &str, private_key: &SecretKey<Secp256k1>) -> Vec<u8> {
        let (signature, recovery): (k256::ecdsa::Signature, _) =
            SigningKey::from_bytes(&private_key.to_bytes())
                .unwrap()
                .sign_digest_recoverable(Keccak256::new_with_prefix(eip191_bytes(message)))
                .unwrap();
        let signature = signature.to_bytes();
        // need for +27 is mentioned in EIP-1271 reference implementation
        [&signature[..], &[recovery.to_byte() + 27]].concat()
    }

    fn message_hash(message: &str) -> [u8; 32] {
        Keccak256::new_with_prefix(eip191_bytes(message)).finalize()[..]
            .try_into()
            .unwrap()
    }

    #[tokio::test]
    async fn test_eip1271_pass() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let contract_address = deploy_contract(&rpc_url, &private_key).await;

        let message = "xxx";
        let signature = sign_message(message, &private_key);

        assert!(
            verify_eip1271(signature, contract_address, &message_hash(message), rpc_url)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_eip1271_fail() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let contract_address = deploy_contract(&rpc_url, &private_key).await;

        let message = "xxx";
        let mut signature = sign_message(message, &private_key);
        signature[0] = signature[0].wrapping_add(1);

        assert!(matches!(
            verify_eip1271(signature, contract_address, &message_hash(message), rpc_url).await,
            Err(CacaoError::Verification)
        ));
    }

    #[tokio::test]
    async fn test_eip1271_fail_wrong_signer() {
        let (anvil, rpc_url, private_key) = spawn_anvil().await;
        let contract_address = deploy_contract(&rpc_url, &private_key).await;

        let message = "xxx";
        let signature = sign_message(message, &anvil.keys()[1]);

        assert!(matches!(
            verify_eip1271(signature, contract_address, &message_hash(message), rpc_url).await,
            Err(CacaoError::Verification)
        ));
    }

    #[tokio::test]
    async fn test_eip1271_fail_wrong_contract_address() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let mut contract_address = deploy_contract(&rpc_url, &private_key).await;

        contract_address.0[0] = contract_address.0[0].wrapping_add(1);

        let message = "xxx";
        let signature = sign_message(message, &private_key);

        assert!(matches!(
            verify_eip1271(signature, contract_address, &message_hash(message), rpc_url).await,
            Err(CacaoError::Verification)
        ));
    }
}
