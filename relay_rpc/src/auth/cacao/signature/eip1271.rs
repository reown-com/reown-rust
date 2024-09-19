use {
    super::CacaoError,
    alloy::{
        primitives::Address,
        providers::{network::Ethereum, Provider, ReqwestProvider},
        rpc::types::{TransactionInput, TransactionRequest},
        sol,
        sol_types::SolCall,
    },
    url::Url,
};

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
) -> Result<(), CacaoError> {
    let provider = ReqwestProvider::<Ethereum>::new_http(provider);

    let call_request = TransactionRequest::default()
        .to(address)
        .input(TransactionInput::new(
            isValidSignatureCall {
                _hash: hash.into(),
                _signature: signature.into(),
            }
            .abi_encode()
            .into(),
        ));

    let result = provider.call(&call_request).await.map_err(|e| {
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
            Ok(())
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
        crate::auth::cacao::signature::{
            eip191::eip191_bytes,
            strip_hex_prefix,
            test_helpers::{
                deploy_contract,
                message_hash,
                sign_message,
                spawn_anvil,
                EIP1271_MOCK_CONTRACT,
            },
        },
        alloy::primitives::address,
        k256::ecdsa::SigningKey,
        sha3::{Digest, Keccak256},
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
        verify_eip1271(signature, address, hash, provider)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_eip1271_pass() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let contract_address = deploy_contract(
            &rpc_url,
            &private_key,
            EIP1271_MOCK_CONTRACT,
            Some(&Address::from_private_key(&private_key).to_string()),
        )
        .await;

        let message = "xxx";
        let signature = sign_message(message, &private_key);

        verify_eip1271(signature, contract_address, &message_hash(message), rpc_url)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_eip1271_wrong_signature() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let contract_address = deploy_contract(
            &rpc_url,
            &private_key,
            EIP1271_MOCK_CONTRACT,
            Some(&Address::from_private_key(&private_key).to_string()),
        )
        .await;

        let message = "xxx";
        let mut signature = sign_message(message, &private_key);
        *signature.first_mut().unwrap() = signature.first().unwrap().wrapping_add(1);

        assert!(matches!(
            verify_eip1271(signature, contract_address, &message_hash(message), rpc_url).await,
            Err(CacaoError::Verification)
        ));
    }

    #[tokio::test]
    async fn test_eip1271_fail_wrong_signer() {
        let (anvil, rpc_url, private_key) = spawn_anvil().await;
        let contract_address = deploy_contract(
            &rpc_url,
            &private_key,
            EIP1271_MOCK_CONTRACT,
            Some(&Address::from_private_key(&private_key).to_string()),
        )
        .await;

        let message = "xxx";
        let signature = sign_message(
            message,
            &SigningKey::from_bytes(&anvil.keys().get(1).unwrap().to_bytes()).unwrap(),
        );

        assert!(matches!(
            verify_eip1271(signature, contract_address, &message_hash(message), rpc_url).await,
            Err(CacaoError::Verification)
        ));
    }

    #[tokio::test]
    async fn test_eip1271_fail_wrong_contract_address() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let mut contract_address = deploy_contract(
            &rpc_url,
            &private_key,
            EIP1271_MOCK_CONTRACT,
            Some(&Address::from_private_key(&private_key).to_string()),
        )
        .await;

        *contract_address.0.first_mut().unwrap() =
            contract_address.0.first().unwrap().wrapping_add(1);

        let message = "xxx";
        let signature = sign_message(message, &private_key);

        assert!(matches!(
            verify_eip1271(signature, contract_address, &message_hash(message), rpc_url).await,
            Err(CacaoError::Verification)
        ));
    }

    #[tokio::test]
    async fn test_eip1271_wrong_message() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let contract_address = deploy_contract(
            &rpc_url,
            &private_key,
            EIP1271_MOCK_CONTRACT,
            Some(&Address::from_private_key(&private_key).to_string()),
        )
        .await;

        let message = "xxx";
        let signature = sign_message(message, &private_key);

        let message2 = "yyy";
        assert!(matches!(
            verify_eip1271(
                signature,
                contract_address,
                &message_hash(message2),
                rpc_url
            )
            .await,
            Err(CacaoError::Verification)
        ));
    }
}
