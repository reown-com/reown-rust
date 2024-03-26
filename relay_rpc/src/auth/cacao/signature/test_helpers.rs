use {
    super::eip191::eip191_bytes,
    alloy_node_bindings::{Anvil, AnvilInstance},
    alloy_primitives::Address,
    k256::ecdsa::SigningKey,
    regex::Regex,
    sha2::Digest,
    sha3::Keccak256,
    std::process::Stdio,
    tokio::process::Command,
    url::Url,
};

pub async fn spawn_anvil() -> (AnvilInstance, Url, SigningKey) {
    let anvil = Anvil::new().spawn();
    let provider = anvil.endpoint().parse().unwrap();
    let private_key = anvil.keys().first().unwrap().clone();
    (
        anvil,
        provider,
        SigningKey::from_bytes(&private_key.to_bytes()).unwrap(),
    )
}

pub async fn deploy_contract(rpc_url: &Url, private_key: &SigningKey) -> Address {
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
