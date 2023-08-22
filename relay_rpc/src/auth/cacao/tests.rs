use crate::auth::cacao::Cacao;

/// Test that we can verify a Cacao
#[test]
fn verify_success() {
    let cacao_serialized = r#"{
      "h": {
        "t": "eip4361"
      },
      "p": {
        "iss": "did:pkh:eip155:1:0x262f4f5DC82ad9b803680F07Da7d901D4F71d8D1",
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
        "s": "0xf2f0e5dc8875ef1e3d40472078b06ebe4af5fc832e464338996fb0d3134cde7613bc36416519e8dd8959655f0e89c6b7a9de55f7c95f43e8d2240f89939ed7171c"
      }
    } "#;
    let cacao: Cacao = serde_json::from_str(cacao_serialized).unwrap();
    println!("{}", cacao.siwe_message().unwrap());
    let result = cacao.verify();
    assert!(result.is_ok());
    assert!(result.map_err(|_| false).unwrap());

    let identity_key = cacao.p.identity_key();
    assert!(identity_key.is_ok());
}

/// Test that we can verify a Cacao with a statement
#[test]
fn verify_success_statement() {
    let cacao_serialized = r#"{
      "h": {
        "t": "eip4361"
      },
      "p": {
        "iss": "did:pkh:eip155:1:0x262f4f5DC82ad9b803680F07Da7d901D4F71d8D1",
        "domain": "http://10.0.2.2:8080",
        "aud": "http://10.0.2.2:8080",
        "statement": "Test statement",
        "version": "1",
        "nonce": "[B@c3772c7",
        "iat": "2023-01-17T12:15:05+01:00",
        "resources": [
          "did:key:z6MkkG9nM8ksS37sq5mgeoCn5kihLkWANcm9pza5WTkq3tWZ"
        ]
      },
      "s": {
        "t": "eip191",
        "s": "0xafedb7505846dc691a4f3f70266624a91a232d68ec61454f4426e016bcb0483773296097687429c47af82b5bf16324ec4ede13e67aee5b4597c9d34b3af0e3681c"
      }
    } "#;
    let cacao: Cacao = serde_json::from_str(cacao_serialized).unwrap();
    println!("{}", cacao.siwe_message().unwrap());
    let result = cacao.verify();
    assert!(result.is_ok());
    assert!(result.map_err(|_| false).unwrap());

    let identity_key = cacao.p.identity_key();
    assert!(identity_key.is_ok());
}

/// Test that we can verify a Cacao with uppercase address
#[test]
fn without_lowercase_address_verify_success() {
    let cacao_serialized = r#"{"h":{"t":"eip4361"},"p":{"iss":"did:pkh:eip155:1:0xbD4D1935165012e7D29919dB8717A5e670a1a5b1","domain":"https://staging.keys.walletconnect.com","aud":"https://staging.keys.walletconnect.com","version":"1","nonce":"07487c09be5535dcbc341d8e35e5c9b4d3539a802089c42c5b1172dd9ed63c64","iat":"2023-01-25T15:08:36.846Z","statement":"Test","resources":["did:key:451cf9b97c64fcca05fbb0d4c40b886c94133653df5a2b6bd97bd29a0bbcdb37"]},"s":{"t":"eip191","s":"0x8496ad1dd1ddd5cb78ac26b62a6bd1c6cfff703ea3b11a9da29cfca112357ace75cac8ee28d114f9e166a6935ee9ed83151819a9e0ee738a0547116b1d978e351b"}}"#;
    let cacao: Cacao = serde_json::from_str(cacao_serialized).unwrap();
    let result = cacao.verify();
    assert!(result.is_ok());
    assert!(result.map_err(|_| false).unwrap());

    let identity_key = cacao.p.identity_key();
    assert!(identity_key.is_ok());
}
