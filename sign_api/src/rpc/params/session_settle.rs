//! https://specs.walletconnect.com/2.0/specs/clients/sign/rpc-methods
//! #wc_sessionsettle

use {
    super::{IrnMetadata, Metadata, Relay, SettleNamespaces},
    serde::{Deserialize, Serialize},
};

pub(super) const IRN_REQUEST_METADATA: IrnMetadata = IrnMetadata {
    tag: 1102,
    ttl: 300,
    prompt: false,
};

pub(super) const IRN_RESPONSE_METADATA: IrnMetadata = IrnMetadata {
    tag: 1103,
    ttl: 300,
    prompt: false,
};

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Controller {
    pub public_key: String,
    pub metadata: Metadata,
}

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionSettleRequest {
    pub relay: Relay,
    pub controller: Controller,
    pub namespaces: SettleNamespaces,
    /// Unix timestamp.
    ///
    /// Expiry should be between .now() + TTL.
    pub expiry: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::tests::param_serde_test;
    use anyhow::Result;

    #[test]
    fn test_serde_session_settle_request() -> Result<()> {
        // Coppied from `session_propose` and adjusted slightly.
        let json = r#"
        {
            "relay": {
                "protocol": "irn"
            },
            "controller": {
                "publicKey": "a3ad5e26070ddb2809200c6f56e739333512015bceeadbb8ea1731c4c7ddb207",
                "metadata": {
                    "description": "React App for WalletConnect",
                    "url": "http://localhost:3000",
                    "icons": [
                        "https://avatars.githubusercontent.com/u/37784886"
                    ],
                    "name": "React App"
                }
            },
            "namespaces": {
                "eip155": {
                    "accounts": [
                        "eip155:5:0xBA5BA3955463ADcc7aa3E33bbdfb8A68e0933dD8"
                    ],
                    "methods": [
                        "eth_sendTransaction",
                        "eth_sign",
                        "eth_signTransaction",
                        "eth_signTypedData",
                        "personal_sign"
                    ],
                    "events": [
                        "accountsChanged",
                        "chainChanged"
                    ]
                }
            },
            "expiry": 1675734962
        }
        "#;

        param_serde_test::<SessionSettleRequest>(json)
    }
}
