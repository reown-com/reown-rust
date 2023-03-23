use crate::auth::cacao::Cacao;

/// Test that we can verify a Cacao
#[test]
fn cacao_verify_success() {
    let cacao_serialized = r#"{
      "h": {
        "t": "eip4361"
      },
      "p": {
        "iss": "did:pkh:eip155:1:0xf457f233ab23f863cabc383ebb37b29d8929a17a",
        "domain": "http://10.0.2.2:8080",
        "aud": "http://10.0.2.2:8080",
        "version": "1",
        "nonce": "[B@c3772c7",
        "iat": "2023-01-17T12:15:05+01:00",
        "resources": [
          "did:key:z6MkkG9nM8ksS37sq5mgeoCn5kihLkWANcm9pza5WTkq3tWZ"
        ]
      },
      "s": {
        "t": "eip191",
        "s": "0x1b39982707c70c95f4676e7386052a07b47ecc073b3e9cf47b64b579687d3f68181d48fa9e926ad591ba6954f1a70c597d0772a800bed5fa906384fcd83bcf4f1b"
      }
    } "#;
    let cacao: Cacao = serde_json::from_str(cacao_serialized).unwrap();
    let result = cacao.verify();
    assert!(result.is_ok());
    assert!(result.map_err(|_| false).unwrap());

    let identity_key = cacao.p.identity_key();
    assert!(identity_key.is_ok());
}

/// Test that we can verify a Cacao with uppercase address
#[test]
fn cacao_without_lowercase_address_verify_success() {
    let cacao_serialized = r#"{"h":{"t":"eip4361"},"p":{"iss":"did:pkh:eip155:1:0xbD4D1935165012e7D29919dB8717A5e670a1a5b1","domain":"https://staging.keys.walletconnect.com","aud":"https://staging.keys.walletconnect.com","version":"1","nonce":"07487c09be5535dcbc341d8e35e5c9b4d3539a802089c42c5b1172dd9ed63c64","iat":"2023-01-25T15:08:36.846Z","statement":"Test","resources":["did:key:451cf9b97c64fcca05fbb0d4c40b886c94133653df5a2b6bd97bd29a0bbcdb37"]},"s":{"t":"eip191","s":"0x8496ad1dd1ddd5cb78ac26b62a6bd1c6cfff703ea3b11a9da29cfca112357ace75cac8ee28d114f9e166a6935ee9ed83151819a9e0ee738a0547116b1d978e351b"}}"#;
    let cacao: Cacao = serde_json::from_str(cacao_serialized).unwrap();
    let result = cacao.verify();
    assert!(result.is_ok());
    assert!(result.map_err(|_| false).unwrap());

    let identity_key = cacao.p.identity_key();
    assert!(identity_key.is_ok());
}
