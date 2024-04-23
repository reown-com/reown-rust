use {
    serde_json::Value,
    std::process::{Command, Stdio},
};

fn main() {
    println!("cargo::rerun-if-changed=relay_rpc/contracts/");
    #[cfg(feature = "cacao")]
    compile_contracts();

    #[cfg(feature = "cacao")]
    extract_eip6492_bytecode();
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

fn extract_eip6492_bytecode() {
    const EIP6492_FILE: &str = "../target/.forge/out/Eip6492.sol/ValidateSigOffchain.json";
    const EIP6492_BYTECODE_FILE: &str =
        "../target/.forge/out/Eip6492.sol/ValidateSigOffchain.bytecode";

    let contents = serde_json::from_slice::<Value>(&std::fs::read(EIP6492_FILE).unwrap()).unwrap();
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
    std::fs::write(EIP6492_BYTECODE_FILE, bytecode).unwrap();
}
