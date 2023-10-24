//! https://specs.walletconnect.com/2.0/specs/clients/sign/rpc-methods
//! #wc_sessionpropose

use {
    super::{IrnMetadata, Metadata, ProposeNamespaces, Relay},
    serde::{Deserialize, Serialize},
};

pub(super) const IRN_REQUEST_METADATA: IrnMetadata = IrnMetadata {
    tag: 1100,
    ttl: 300,
    prompt: true,
};

pub(super) const IRN_RESPONSE_METADATA: IrnMetadata = IrnMetadata {
    tag: 1101,
    ttl: 300,
    prompt: false,
};

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Proposer {
    pub public_key: String,
    pub metadata: Metadata,
}

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionProposeRequest {
    pub relays: Vec<Relay>,
    pub proposer: Proposer,
    pub required_namespaces: ProposeNamespaces,
}

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionProposeResponse {
    pub relay: Relay,
    pub responder_public_key: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::tests::param_serde_test;
    use anyhow::Result;

    #[test]
    fn test_serde_session_propose_request() -> Result<()> {
        // https://specs.walletconnect.com/2.0/specs/clients/sign/
        // session-events#session_propose
        let json = r#"
        {
            "relays": [
                {
                    "protocol": "irn"
                }
            ],
            "proposer": {
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
            "requiredNamespaces": {
                "eip155": {
                    "chains": [
                        "eip155:5"
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
            }
        }
        "#;

        param_serde_test::<SessionProposeRequest>(json)
    }
}
