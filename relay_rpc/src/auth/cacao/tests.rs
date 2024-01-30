use crate::auth::cacao::Cacao;

/// Test that we can verify a deprecated Cacao.
#[tokio::test]
async fn cacao_verify_success() {
    let cacao_serialized = r#"{
      "h": {
        "t": "eip4361"
      },
      "p": {
        "iss": "did:pkh:eip155:1:0xB1bad80be351061Db2F726D2dDe28E0Ebbb88D30",
        "domain": "keys.walletconnect.com",
        "aud": "https://keys.walletconnect.com",
        "version": "1",
        "nonce": "2c586f5025cb20094329ccd83684e2b192bebb2a3f83fc91b0b27aa817fd24de",
        "iat": "2023-05-17T14:22:32+02:00",
        "resources": [
          "did:key:z6MkhoV7JnKEFgwai4R1ui14xcPDnqVFZ3a9dUNM3fE3z3Nf"
        ]
      },
      "s": {
        "t": "eip191",
        "s": "0x991f379195564ba1d131c53cc9b3cf13c03e3a8111f502fd40ca12e1d04d98ea58531295c48f852c9c35a938c778f52a2c994f109fc0e94cc4e16f62d41d54371c"
      }
    }"#;
    let cacao: Cacao = serde_json::from_str(cacao_serialized).unwrap();
    let result = cacao.verify(|_| None).await;
    assert!(result.is_ok());
    assert!(result.map_err(|_| false).unwrap());

    let identity_key = cacao.p.identity_key();
    assert!(identity_key.is_ok());
    assert_eq!(
        identity_key.unwrap(),
        "z6MkhoV7JnKEFgwai4R1ui14xcPDnqVFZ3a9dUNM3fE3z3Nf"
    )
}

/// Test that we can verify a updated Cacao.
#[tokio::test]
async fn cacao_verify_success_identity_in_audience() {
    let cacao_serialized = r#"{
        "h": {
            "t": "eip4361"
        },
        "p": {
            "iss": "did:pkh:eip155:1:0xdFe7d0E324ed017a74aE311E9236E6CaDB24176b",
            "domain": "com.walletconnect.sample.web3inbox",
            "aud": "did:key:z6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht7",
            "version": "1",
            "nonce": "6c9435d868ce15e0a1b0987a61a975e8c0edda17054840548dabf0a3c55cf5e4",
            "iat": "2023-09-07T11:04:23+02:00",
            "statement": "I further authorize this DAPP to send and receive messages on my behalf for this domain and manage my identity at identity.walletconnect.com.",
            "resources": [
                "identity.walletconnect.com"
            ]
        },
        "s": {
            "t": "eip191",
            "s": "0x18b8dd2595930bd4bcd8066ad9fca5c54aaab20d2ec1cf46ff90baa5a91acad80f064a2f533d9dfc75928958a1da8e4f6755e14cab325a40a3a51e4bd6f2a1c91b"
        }
    }"#;
    let cacao: Cacao = serde_json::from_str(cacao_serialized).unwrap();
    let result = cacao.verify(|_| None).await;
    assert!(result.is_ok());
    assert!(result.map_err(|_| false).unwrap());

    let identity_key = cacao.p.identity_key();
    assert!(identity_key.is_ok());
    assert_eq!(
        identity_key.unwrap(),
        "z6MkvjNoiz9AXGH1igzrtB54US5hE9bZPQm1ryKGkCLwWht7"
    )
}

/// Test that we can verify a Cacao
#[tokio::test]
async fn cacao_verify_failure() {
    let cacao_serialized = r#"{
      "h": {
        "t": "eip4361"
      },
      "p": {
        "iss": "did:pkh:eip155:1:0xF5dA9A1Aa622903ae73f5eFE46485531913202AF",
        "domain": "keys.walletconnect.com",
        "aud": "https://keys.walletconnect.com",
        "version": "1",
        "nonce": "0d98d4e5d8c19d4cff09cd25f1863bca650d2b4009bd62f04dff7171438c4773",
        "iat": "2023-05-17T14:14:24+02:00",
        "resources": [
          "did:key:z6MkgzojB48jpTcLTatSCRHNpoMRvQbz8r13UJ1KyteHjEu9"
        ]
      },
      "s": {
        "t": "eip191",
        "s": "0x726caf0b066fd857889fa73a8b04cbe249161c37a9342854ec92258a85a91ca5720d6d61afe45c7a54f42373ab1c90d888257637a938af5d9f242adad43b204d1b"
      }
    }"#;
    let cacao: Cacao = serde_json::from_str(cacao_serialized).unwrap();
    let result = cacao.verify(|_| None).await;
    assert!(result.is_err());
}
