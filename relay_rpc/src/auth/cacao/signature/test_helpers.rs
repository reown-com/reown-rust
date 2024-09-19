use {
    super::eip191::eip191_bytes,
    alloy::{
        node_bindings::{Anvil, AnvilInstance},
        primitives::Address,
    },
    k256::ecdsa::SigningKey,
    regex::Regex,
    sha2::Digest,
    sha3::Keccak256,
    std::process::Stdio,
    tokio::process::Command,
    url::Url,
};

fn format_foundry_dir(path: &str) -> String {
    format!(
        "{}/../../../../.foundry/{}",
        std::env::var("OUT_DIR").unwrap(),
        path
    )
}

pub async fn spawn_anvil() -> (AnvilInstance, Url, SigningKey) {
    let anvil = Anvil::at(format_foundry_dir("bin/anvil")).spawn();
    let provider = anvil.endpoint().parse().unwrap();
    let private_key = anvil.keys().first().unwrap().clone();
    (
        anvil,
        provider,
        SigningKey::from_bytes(&private_key.to_bytes()).unwrap(),
    )
}

pub const EIP1271_MOCK_CONTRACT: &str = "Eip1271Mock";
pub const CREATE2_CONTRACT: &str = "Create2";

pub async fn deploy_contract(
    rpc_url: &Url,
    private_key: &SigningKey,
    contract_name: &str,
    constructor_arg: Option<&str>,
) -> Address {
    let key_encoded = data_encoding::HEXLOWER_PERMISSIVE.encode(&private_key.to_bytes());
    let cache_folder = format_foundry_dir("forge/cache");
    let out_folder = format_foundry_dir("forge/out");
    let mut args = vec![
        "create",
        "--contracts=relay_rpc/contracts",
        contract_name,
        "--rpc-url",
        rpc_url.as_str(),
        "--private-key",
        &key_encoded,
        "--cache-path",
        &cache_folder,
        "--out",
        &out_folder,
    ];
    if let Some(arg) = constructor_arg {
        args.push("--constructor-args");
        args.push(arg);
    }
    let output = Command::new(format_foundry_dir("bin/forge"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .await
        .unwrap();
    println!("forge status: {:?}", output.status);
    let stdout = String::from_utf8(output.stdout).unwrap();
    println!("forge stdout: {stdout:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    println!("forge stderr: {stderr:?}");
    assert!(output.status.success());
    let (_, [contract_address]) = Regex::new("Deployed to: (0x[0-9a-fA-F]+)")
        .unwrap()
        .captures(&stdout)
        .unwrap()
        .extract();
    contract_address.parse().unwrap()
}

pub fn sign_message(message: &str, private_key: &SigningKey) -> Vec<u8> {
    let (signature, recovery): (k256::ecdsa::Signature, _) = private_key
        .sign_digest_recoverable(message_hash_internal(message))
        .unwrap();
    let signature = signature.to_bytes();
    // need for +27 is mentioned in EIP-1271 reference implementation
    [&signature[..], &[recovery.to_byte() + 27]].concat()
}

pub fn message_hash_internal(message: &str) -> Keccak256 {
    Keccak256::new_with_prefix(eip191_bytes(message))
}

pub fn message_hash(message: &str) -> [u8; 32] {
    message_hash_internal(message).finalize()[..]
        .try_into()
        .unwrap()
}
