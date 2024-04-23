use {
    crate::auth::cacao::CacaoError,
    alloy_primitives::Address,
    alloy_provider::{network::Ethereum, Provider, ReqwestProvider},
    alloy_rpc_types::{TransactionInput, TransactionRequest},
    alloy_sol_types::{sol, SolConstructor},
    url::Url,
};

pub const EIP6492: &str = "eip6492";

// https://eips.ethereum.org/EIPS/eip-6492
const MAGIC_VALUE: u8 = 0x01;
sol! {
  contract ValidateSigOffchain {
    constructor (address _signer, bytes32 _hash, bytes memory _signature);
  }
}
const VALIDATE_SIG_OFFCHAIN_BYTECODE: &[u8] =
    include_bytes!("../../../../../target/.forge/out/Eip6492.sol/ValidateSigOffchain.bytecode");

pub async fn verify_eip6492(
    signature: Vec<u8>,
    address: Address,
    hash: &[u8; 32],
    provider: Url,
) -> Result<(), CacaoError> {
    let provider = ReqwestProvider::<Ethereum>::new_http(provider);

    let call = ValidateSigOffchain::constructorCall {
        _signer: address,
        _hash: hash.into(),
        _signature: signature.into(),
    };
    let bytes = VALIDATE_SIG_OFFCHAIN_BYTECODE
        .iter()
        .cloned()
        .chain(call.abi_encode())
        .collect::<Vec<u8>>();
    let transaction_request =
        TransactionRequest::default().input(TransactionInput::new(bytes.into()));

    let result = provider
        .call(&transaction_request, Default::default())
        .await
        .map_err(CacaoError::Eip6492Internal)?;

    let magic = result.first();
    if let Some(magic) = magic {
        if magic == &MAGIC_VALUE {
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
        crate::auth::cacao::signature::test_helpers::{
            deploy_contract,
            message_hash,
            sign_message,
            spawn_anvil,
        },
        k256::ecdsa::SigningKey,
    };

    #[tokio::test]
    async fn test_eip191_pass() {
        let (_anvil, rpc_url, _private_key) = spawn_anvil().await;

        let private_key = SigningKey::random(&mut rand::thread_rng());
        let message = "xxx";
        let signature = sign_message(message, &private_key);
        let address = Address::from_private_key(&private_key);
        verify_eip6492(signature, address, &message_hash(message), rpc_url)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_eip191_wrong_signature() {
        let (_anvil, rpc_url, _private_key) = spawn_anvil().await;

        let private_key = SigningKey::random(&mut rand::thread_rng());
        let message = "xxx";
        let mut signature = sign_message(message, &private_key);
        *signature.first_mut().unwrap() = signature.first().unwrap().wrapping_add(1);
        let address = Address::from_private_key(&private_key);
        assert!(
            verify_eip6492(signature, address, &message_hash(message), rpc_url)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_eip191_wrong_address() {
        let (_anvil, rpc_url, _private_key) = spawn_anvil().await;

        let private_key = SigningKey::random(&mut rand::thread_rng());
        let message = "xxx";
        let signature = sign_message(message, &private_key);
        let mut address = Address::from_private_key(&private_key);
        *address.0.first_mut().unwrap() = address.0.first().unwrap().wrapping_add(1);
        assert!(
            verify_eip6492(signature, address, &message_hash(message), rpc_url)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_eip191_wrong_message() {
        let (_anvil, rpc_url, _private_key) = spawn_anvil().await;

        let private_key = SigningKey::random(&mut rand::thread_rng());
        let message = "xxx";
        let signature = sign_message(message, &private_key);
        let address = Address::from_private_key(&private_key);
        let message2 = "yyy";
        assert!(
            verify_eip6492(signature, address, &message_hash(message2), rpc_url)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_eip1271_pass() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let contract_address = deploy_contract(&rpc_url, &private_key).await;

        let message = "xxx";
        let signature = sign_message(message, &private_key);

        verify_eip6492(signature, contract_address, &message_hash(message), rpc_url)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_eip1271_wrong_signature() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let contract_address = deploy_contract(&rpc_url, &private_key).await;

        let message = "xxx";
        let mut signature = sign_message(message, &private_key);
        *signature.first_mut().unwrap() = signature.first().unwrap().wrapping_add(1);

        assert!(matches!(
            verify_eip6492(signature, contract_address, &message_hash(message), rpc_url).await,
            Err(CacaoError::Verification)
        ));
    }

    #[tokio::test]
    async fn test_eip1271_fail_wrong_signer() {
        let (anvil, rpc_url, private_key) = spawn_anvil().await;
        let contract_address = deploy_contract(&rpc_url, &private_key).await;

        let message = "xxx";
        let signature = sign_message(
            message,
            &SigningKey::from_bytes(&anvil.keys().get(1).unwrap().to_bytes()).unwrap(),
        );

        assert!(matches!(
            verify_eip6492(signature, contract_address, &message_hash(message), rpc_url).await,
            Err(CacaoError::Verification)
        ));
    }

    #[tokio::test]
    async fn test_eip1271_fail_wrong_contract_address() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let mut contract_address = deploy_contract(&rpc_url, &private_key).await;

        *contract_address.0.first_mut().unwrap() =
            contract_address.0.first().unwrap().wrapping_add(1);

        let message = "xxx";
        let signature = sign_message(message, &private_key);

        assert!(matches!(
            verify_eip6492(signature, contract_address, &message_hash(message), rpc_url).await,
            Err(CacaoError::Verification)
        ));
    }

    #[tokio::test]
    async fn test_eip1271_wrong_message() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let contract_address = deploy_contract(&rpc_url, &private_key).await;

        let message = "xxx";
        let signature = sign_message(message, &private_key);

        let message2 = "yyy";
        assert!(matches!(
            verify_eip6492(
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
