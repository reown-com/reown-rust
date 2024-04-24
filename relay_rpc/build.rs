use {
    serde_json::Value,
    std::process::{Command, Stdio},
};

fn main() {
    println!("cargo::rerun-if-changed=relay_rpc/contracts/");
    #[cfg(feature = "cacao")]
    compile_contracts();

    #[cfg(feature = "cacao")]
    extract_bytecodes();
}

fn compile_contracts() {
    let output = Command::new("forge")
        .args([
            "build",
            "--contracts=relay_rpc/contracts",
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
        .unwrap();
    println!("forge status: {:?}", output.status);
    let stdout = String::from_utf8(output.stdout).unwrap();
    println!("forge stdout: {stdout:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    println!("forge stderr: {stderr:?}");
    assert!(output.status.success());
}

const EIP6492_FILE: &str = "../target/.forge/out/Eip6492.sol/ValidateSigOffchain.json";
const EIP6492_BYTECODE_FILE: &str = "../target/.forge/out/Eip6492.sol/ValidateSigOffchain.bytecode";
const EIP1271_MOCK_FILE: &str = "../target/.forge/out/Eip1271Mock.sol/Eip1271Mock.json";
const EIP1271_MOCK_BYTECODE_FILE: &str =
    "../target/.forge/out/Eip1271Mock.sol/Eip1271Mock.bytecode";
fn extract_bytecodes() {
    extract_bytecode(EIP6492_FILE, EIP6492_BYTECODE_FILE);
    extract_bytecode(EIP1271_MOCK_FILE, EIP1271_MOCK_BYTECODE_FILE);
}

fn extract_bytecode(input_file: &str, output_file: &str) {
    let contents = serde_json::from_slice::<Value>(&std::fs::read(input_file).unwrap()).unwrap();
    let bytecode = contents
        .get("bytecode")
        .unwrap()
        .get("object")
        .unwrap()
        .as_str()
        .unwrap()
        .strip_prefix("0x")
        .unwrap();
    let bytecode = hex::decode(bytecode).unwrap();
    std::fs::write(output_file, bytecode).unwrap();
}
