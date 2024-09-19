use {
    crate::auth::cacao::CacaoError,
    alloy::{
        primitives::Address,
        providers::{network::Ethereum, Provider, ReqwestProvider},
        rpc::types::{TransactionInput, TransactionRequest},
        sol,
        sol_types::SolConstructor,
    },
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
const VALIDATE_SIG_OFFCHAIN_BYTECODE: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/../../../../.foundry/forge/out/Eip6492.sol/ValidateSigOffchain.bytecode"
));

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

    let result = provider.call(&transaction_request).await.map_err(|e| {
        if let Some(error_response) = e.as_error_resp() {
            if error_response.message == "execution reverted" {
                CacaoError::Verification
            } else {
                CacaoError::Eip6492Internal(e)
            }
        } else {
            CacaoError::Eip6492Internal(e)
        }
    })?;

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
        crate::auth::cacao::signature::{
            strip_hex_prefix,
            test_helpers::{
                deploy_contract,
                message_hash,
                sign_message,
                spawn_anvil,
                CREATE2_CONTRACT,
                EIP1271_MOCK_CONTRACT,
            },
        },
        alloy::{
            primitives::{address, b256, Uint},
            sol_types::{SolCall, SolValue},
        },
        k256::ecdsa::SigningKey,
    };

    // Manual test. Paste address, signature, message, and project ID to verify
    // function
    #[tokio::test]
    #[ignore]
    async fn test_eip6492_manual() {
        let address = address!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        let message = "xxx";
        let signature = "xxx";

        let signature = data_encoding::HEXLOWER_PERMISSIVE
            .decode(strip_hex_prefix(signature).as_bytes())
            .map_err(|_| CacaoError::Verification)
            .unwrap();
        let provider = "https://rpc.walletconnect.com/v1?chainId=eip155:1&projectId=xxx"
            .parse()
            .unwrap();
        verify_eip6492(signature, address, &message_hash(message), provider)
            .await
            .unwrap();
    }

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
        let contract_address = deploy_contract(
            &rpc_url,
            &private_key,
            EIP1271_MOCK_CONTRACT,
            Some(&Address::from_private_key(&private_key).to_string()),
        )
        .await;

        let message = "xxx";
        let signature = sign_message(message, &private_key);

        verify_eip6492(signature, contract_address, &message_hash(message), rpc_url)
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
            verify_eip6492(signature, contract_address, &message_hash(message), rpc_url).await,
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
            verify_eip6492(signature, contract_address, &message_hash(message), rpc_url).await,
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
            verify_eip6492(signature, contract_address, &message_hash(message), rpc_url).await,
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

    const EIP1271_MOCK_BYTECODE: &[u8] = include_bytes!(concat!(
        env!("OUT_DIR"),
        "/../../../../.foundry/forge/out/Eip1271Mock.sol/Eip1271Mock.bytecode"
    ));
    const EIP6492_MAGIC_BYTES: [u16; 16] = [
        0x6492, 0x6492, 0x6492, 0x6492, 0x6492, 0x6492, 0x6492, 0x6492, 0x6492, 0x6492, 0x6492,
        0x6492, 0x6492, 0x6492, 0x6492, 0x6492,
    ];
    sol! {
        contract Eip1271Mock {
            address owner_eoa;

            constructor(address owner_eoa) {
                owner_eoa = owner_eoa;
            }
        }
    }

    sol! {
        contract Create2 {
            function deploy(uint256 amount, bytes32 salt, bytes memory bytecode) external payable returns (address addr);
        }
    }

    fn predeploy_signature(
        owner_eoa: Address,
        create2_factory_address: Address,
        signature: Vec<u8>,
    ) -> (Address, Vec<u8>) {
        let salt = b256!("7c5ea36004851c764c44143b1dcb59679b11c9a68e5f41497f6cf3d480715331");
        let contract_bytecode = EIP1271_MOCK_BYTECODE;
        let contract_constructor = Eip1271Mock::constructorCall { owner_eoa };

        let bytecode = contract_bytecode
            .iter()
            .cloned()
            .chain(contract_constructor.abi_encode())
            .collect::<Vec<u8>>();
        let predeploy_address = create2_factory_address.create2_from_code(salt, bytecode.clone());
        let signature = (
            create2_factory_address,
            Create2::deployCall {
                amount: Uint::ZERO,
                salt,
                bytecode: bytecode.into(),
            }
            .abi_encode(),
            signature,
        )
            .abi_encode_sequence()
            .into_iter()
            .chain(
                EIP6492_MAGIC_BYTES
                    .iter()
                    .flat_map(|&x| x.to_be_bytes().into_iter()),
            )
            .collect::<Vec<u8>>();
        (predeploy_address, signature)
    }

    #[tokio::test]
    async fn test_eip6492_pass() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let create2_factory_address =
            deploy_contract(&rpc_url, &private_key, CREATE2_CONTRACT, None).await;

        let message = "xxx";
        let signature = sign_message(message, &private_key);
        let (predeploy_address, signature) = predeploy_signature(
            Address::from_private_key(&private_key),
            create2_factory_address,
            signature,
        );

        verify_eip6492(
            signature,
            predeploy_address,
            &message_hash(message),
            rpc_url,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_eip6492_wrong_signature() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let create2_factory_address =
            deploy_contract(&rpc_url, &private_key, CREATE2_CONTRACT, None).await;

        let message = "xxx";
        let mut signature = sign_message(message, &private_key);
        *signature.first_mut().unwrap() = signature.first().unwrap().wrapping_add(1);
        let (predeploy_address, signature) = predeploy_signature(
            Address::from_private_key(&private_key),
            create2_factory_address,
            signature,
        );

        assert!(matches!(
            verify_eip6492(
                signature,
                predeploy_address,
                &message_hash(message),
                rpc_url
            )
            .await,
            Err(CacaoError::Verification)
        ));
    }

    #[tokio::test]
    async fn test_eip6492_fail_wrong_signer() {
        let (anvil, rpc_url, private_key) = spawn_anvil().await;
        let create2_factory_address =
            deploy_contract(&rpc_url, &private_key, CREATE2_CONTRACT, None).await;

        let message = "xxx";
        let signature = sign_message(
            message,
            &SigningKey::from_bytes(&anvil.keys().get(1).unwrap().to_bytes()).unwrap(),
        );
        let (predeploy_address, signature) = predeploy_signature(
            Address::from_private_key(&private_key),
            create2_factory_address,
            signature,
        );

        assert!(matches!(
            verify_eip6492(
                signature,
                predeploy_address,
                &message_hash(message),
                rpc_url
            )
            .await,
            Err(CacaoError::Verification)
        ));
    }

    #[tokio::test]
    async fn test_eip6492_fail_wrong_contract_address() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let create2_factory_address =
            deploy_contract(&rpc_url, &private_key, CREATE2_CONTRACT, None).await;

        let message = "xxx";
        let signature = sign_message(message, &private_key);
        let (mut predeploy_address, signature) = predeploy_signature(
            Address::from_private_key(&private_key),
            create2_factory_address,
            signature,
        );

        *predeploy_address.0.first_mut().unwrap() =
            predeploy_address.0.first().unwrap().wrapping_add(1);

        assert!(matches!(
            verify_eip6492(
                signature,
                predeploy_address,
                &message_hash(message),
                rpc_url,
            )
            .await,
            Err(CacaoError::Verification)
        ));
    }

    #[tokio::test]
    async fn test_eip6492_wrong_message() {
        let (_anvil, rpc_url, private_key) = spawn_anvil().await;
        let create2_factory_address =
            deploy_contract(&rpc_url, &private_key, CREATE2_CONTRACT, None).await;

        let message = "xxx";
        let signature = sign_message(message, &private_key);
        let (predeploy_address, signature) = predeploy_signature(
            Address::from_private_key(&private_key),
            create2_factory_address,
            signature,
        );

        let message2 = "yyy";
        assert!(matches!(
            verify_eip6492(
                signature,
                predeploy_address,
                &message_hash(message2),
                rpc_url
            )
            .await,
            Err(CacaoError::Verification)
        ));
    }
}
