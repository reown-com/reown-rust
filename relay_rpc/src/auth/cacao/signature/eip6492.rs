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
  // `bytecode` copied from target/.forge/out/Eip6492.sol/ValidateSigOffchain.json#bytecode.object
  // Copy example contract from the EIP temporarily to `contracts/Eip6492.sol` to generate the bytecode
  #[sol(rpc, bytecode = "0x608060405234801561001057600080fd5b50604051610d8e380380610d8e83398101604081905261002f91610124565b600060405161003d906100dd565b604051809103906000f080158015610059573d6000803e3d6000fd5b5090506000816001600160a01b0316638f0684308686866040518463ffffffff1660e01b815260040161008e939291906101fb565b6020604051808303816000875af11580156100ad573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906100d19190610244565b9050806000526001601ff35b610b208061026e83390190565b634e487b7160e01b600052604160045260246000fd5b60005b8381101561011b578181015183820152602001610103565b50506000910152565b60008060006060848603121561013957600080fd5b83516001600160a01b038116811461015057600080fd5b6020850151604086015191945092506001600160401b038082111561017457600080fd5b818601915086601f83011261018857600080fd5b81518181111561019a5761019a6100ea565b604051601f8201601f19908116603f011681019083821181831017156101c2576101c26100ea565b816040528281528960208487010111156101db57600080fd5b6101ec836020830160208801610100565b80955050505050509250925092565b60018060a01b0384168152826020820152606060408201526000825180606084015261022e816080850160208701610100565b601f01601f191691909101608001949350505050565b60006020828403121561025657600080fd5b8151801515811461026657600080fd5b939250505056fe6080604052348015600f57600080fd5b50610b018061001f6000396000f3fe608060405234801561001057600080fd5b50600436106100415760003560e01c806376be4cea146100465780638f0684301461006d57806398ef1ed814610080575b600080fd5b61005961005436600461070d565b610093565b604051901515815260200160405180910390f35b61005961007b366004610792565b610537565b61005961008e366004610792565b6105b6565b60006001600160a01b0387163b6060827f649264926492649264926492649264926492649264926492649264926492649288886100d16020826107ee565b6100dd928b9290610815565b6100e69161083f565b14905080156101c6576000606089828a6101016020826107ee565b9261010e93929190610815565b81019061011b9190610900565b9550909250905084158061012c5750865b156101bf57600080836001600160a01b03168360405161014c919061099a565b6000604051808303816000865af19150503d8060008114610189576040519150601f19603f3d011682016040523d82523d6000602084013e61018e565b606091505b5091509150816101bc5780604051639d0d6e2d60e01b81526004016101b391906109e2565b60405180910390fd5b50505b5050610200565b87878080601f0160208091040260200160405190810160405280939291908181526020018383808284376000920191909152509294505050505b808061020c5750600083115b1561036f57604051630b135d3f60e11b81526001600160a01b038b1690631626ba7e9061023f908c9086906004016109fc565b602060405180830381865afa925050508015610278575060408051601f3d908101601f1916820190925261027591810190610a15565b60015b6102f4573d8080156102a6576040519150601f19603f3d011682016040523d82523d6000602084013e6102ab565b606091505b50851580156102ba5750600084115b156102d9576102ce8b8b8b8b8b6001610093565b94505050505061052d565b80604051636f2a959960e01b81526004016101b391906109e2565b6001600160e01b03198116630b135d3f60e11b14801581610313575086155b801561031f5750600085115b1561033f576103338c8c8c8c8c6001610093565b9550505050505061052d565b8415801561034a5750825b8015610354575087155b1561036357806000526001601ffd5b945061052d9350505050565b604187146103e55760405162461bcd60e51b815260206004820152603a60248201527f5369676e617475726556616c696461746f72237265636f7665725369676e657260448201527f3a20696e76616c6964207369676e6174757265206c656e67746800000000000060648201526084016101b3565b60006103f46020828a8c610815565b6103fd9161083f565b9050600061040f604060208b8d610815565b6104189161083f565b905060008a8a604081811061042f5761042f610a3f565b919091013560f81c915050601b811480159061044f57508060ff16601c14155b156104b25760405162461bcd60e51b815260206004820152602d60248201527f5369676e617475726556616c696461746f723a20696e76616c6964207369676e60448201526c617475726520762076616c756560981b60648201526084016101b3565b6040805160008152602081018083528e905260ff83169181019190915260608101849052608081018390526001600160a01b038e169060019060a0016020604051602081039080840390855afa158015610510573d6000803e3d6000fd5b505050602060405103516001600160a01b03161496505050505050505b9695505050505050565b604051633b5f267560e11b815260009030906376be4cea906105689088908890889088906001908990600401610a55565b6020604051808303816000875af1158015610587573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906105ab9190610aae565b90505b949350505050565b604051633b5f267560e11b815260009030906376be4cea906105e690889088908890889088908190600401610a55565b6020604051808303816000875af1925050508015610621575060408051601f3d908101601f1916820190925261061e91810190610aae565b60015b610697573d80801561064f576040519150601f19603f3d011682016040523d82523d6000602084013e610654565b606091505b5080516001819003610693578160008151811061067357610673610a3f565b6020910101516001600160f81b031916600160f81b1492506105ae915050565b8082fd5b90506105ae565b6001600160a01b03811681146106b357600080fd5b50565b60008083601f8401126106c857600080fd5b50813567ffffffffffffffff8111156106e057600080fd5b6020830191508360208285010111156106f857600080fd5b9250929050565b80151581146106b357600080fd5b60008060008060008060a0878903121561072657600080fd5b86356107318161069e565b955060208701359450604087013567ffffffffffffffff81111561075457600080fd5b61076089828a016106b6565b9095509350506060870135610774816106ff565b91506080870135610784816106ff565b809150509295509295509295565b600080600080606085870312156107a857600080fd5b84356107b38161069e565b935060208501359250604085013567ffffffffffffffff8111156107d657600080fd5b6107e2878288016106b6565b95989497509550505050565b8181038181111561080f57634e487b7160e01b600052601160045260246000fd5b92915050565b6000808585111561082557600080fd5b8386111561083257600080fd5b5050820193919092039150565b8035602083101561080f57600019602084900360031b1b1692915050565b634e487b7160e01b600052604160045260246000fd5b600082601f83011261088457600080fd5b813567ffffffffffffffff8082111561089f5761089f61085d565b604051601f8301601f19908116603f011681019082821181831017156108c7576108c761085d565b816040528381528660208588010111156108e057600080fd5b836020870160208301376000602085830101528094505050505092915050565b60008060006060848603121561091557600080fd5b83356109208161069e565b9250602084013567ffffffffffffffff8082111561093d57600080fd5b61094987838801610873565b9350604086013591508082111561095f57600080fd5b5061096c86828701610873565b9150509250925092565b60005b83811015610991578181015183820152602001610979565b50506000910152565b600082516109ac818460208701610976565b9190910192915050565b600081518084526109ce816020860160208601610976565b601f01601f19169290920160200192915050565b6020815260006109f560208301846109b6565b9392505050565b8281526040602082015260006105ae60408301846109b6565b600060208284031215610a2757600080fd5b81516001600160e01b0319811681146109f557600080fd5b634e487b7160e01b600052603260045260246000fd5b6001600160a01b03871681526020810186905260a0604082018190528101849052838560c0830137600060c085830181019190915292151560608201529015156080820152601f909201601f1916909101019392505050565b600060208284031215610ac057600080fd5b81516109f5816106ff56fea2646970667358221220304a40774481cc601339bc29f4dc264e3cf712d479399cec7c2b1a9aeeec962964736f6c63430008190033")]
  contract ValidateSigOffchain {
    constructor (address _signer, bytes32 _hash, bytes memory _signature);
  }
}

pub async fn verify_eip6492(
    signature: Vec<u8>,
    address: Address,
    hash: &[u8; 32],
    provider: Url,
) -> Result<(), CacaoError> {
    let provider = ReqwestProvider::<Ethereum>::new_http(provider);

    let call_request = TransactionRequest::default().input(TransactionInput::new(
        [
            ValidateSigOffchain::BYTECODE.clone(),
            ValidateSigOffchain::constructorCall {
                _signer: address,
                _hash: hash.into(),
                _signature: signature.into(),
            }
            .abi_encode()
            .into(),
        ]
        .concat()
        .into(),
    ));

    let result = provider
        .call(&call_request, Default::default())
        .await
        .map_err(CacaoError::Eip1271Internal)?;
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
