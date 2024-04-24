use {
    serde_json::Value,
    std::process::{Command, Stdio},
};

fn main() {
    #[cfg(feature = "cacao")]
    build_contracts();
}

fn build_contracts() {
    println!("cargo::rerun-if-changed=contracts");
    install_foundary();
    compile_contracts();
    extract_bytecodes();
}

fn format_foundry_dir(path: &str) -> String {
    format!(
        "{}/../../../../.foundry/{}",
        std::env::var("OUT_DIR").unwrap(),
        path
    )
}

fn install_foundary() {
    let bin_folder = format_foundry_dir("bin");
    std::fs::remove_dir_all(&bin_folder).ok();
    std::fs::create_dir_all(&bin_folder).unwrap();
    let output = Command::new("bash")
        .args(["-c", &format!("curl https://raw.githubusercontent.com/foundry-rs/foundry/e0ea59cae26d945445d9cf21fdf22f4a18ac5bb2/foundryup/foundryup | FOUNDRY_DIR={} bash", format_foundry_dir(""))])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    println!("foundryup status: {:?}", output.status);
    let stdout = String::from_utf8(output.stdout).unwrap();
    println!("foundryup stdout: {stdout:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    println!("foundryup stderr: {stderr:?}");
    assert!(output.status.success());
}

fn compile_contracts() {
    let output = Command::new(format_foundry_dir("bin/forge"))
        .args([
            "build",
            "--contracts=relay_rpc/contracts",
            "--cache-path",
            &format_foundry_dir("forge/cache"),
            "--out",
            &format_foundry_dir("forge/out"),
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

const EIP6492_FILE: &str = "forge/out/Eip6492.sol/ValidateSigOffchain.json";
const EIP6492_BYTECODE_FILE: &str = "forge/out/Eip6492.sol/ValidateSigOffchain.bytecode";
const EIP1271_MOCK_FILE: &str = "forge/out/Eip1271Mock.sol/Eip1271Mock.json";
const EIP1271_MOCK_BYTECODE_FILE: &str = "forge/out/Eip1271Mock.sol/Eip1271Mock.bytecode";
fn extract_bytecodes() {
    extract_bytecode(
        &format_foundry_dir(EIP6492_FILE),
        &format_foundry_dir(EIP6492_BYTECODE_FILE),
    );
    extract_bytecode(
        &format_foundry_dir(EIP1271_MOCK_FILE),
        &format_foundry_dir(EIP1271_MOCK_BYTECODE_FILE),
    );
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
