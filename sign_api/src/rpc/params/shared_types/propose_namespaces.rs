use {
    regex::Regex,
    serde::{Deserialize, Serialize},
    std::{
        collections::{BTreeMap, BTreeSet},
        ops::Deref,
        sync::OnceLock,
    },
};

/// https://specs.walletconnect.com/2.0/specs/clients/sign/namespaces
///
/// https://chainagnostic.org/CAIPs/caip-2
///
/// chain_id:    namespace + ":" + reference
/// namespace:   [-a-z0-9]{3,8}
/// reference:   [-_a-zA-Z0-9]{1,32}
static CAIP2_REGEX: OnceLock<Regex> = OnceLock::new();
fn get_caip2_regex() -> &'static Regex {
    CAIP2_REGEX.get_or_init(|| {
        Regex::new(r"^(?P<namespace>[-[:alnum:]]{3,8})((?::)(?P<reference>[-_[:alnum:]]{1,32}))?$")
            .expect("invalid regex: unexpected error")
    })
}
/// Errors covering namespace validation errors.
///
/// https://specs.walletconnect.com/2.0/specs/clients/sign/namespaces
/// and some additional variants.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum ProposeNamespaceError {
    #[error("Required chains are not supported: {0}")]
    UnsupportedChains(String),
    #[error("Chains must not be empty")]
    UnsupportedChainsEmpty,
    #[error("Chains must be CAIP-2 compliant: {0}")]
    UnsupportedChainsCaip2(String),
    #[error("Chains must be defined in matching namespace: expected={0}, actual={1}")]
    UnsupportedChainsNamespace(String, String),
    #[error("Required extensions are not supported")]
    UnsupportedExtensions,
    #[error("Required events are not supported: {0}")]
    UnsupportedEvents(String),
    #[error("Required methods are not supported: {0}")]
    UnsupportedMethods(String),
    #[error("Required namespace is not supported: {0}")]
    UnsupportedNamespace(String),
    #[error("Namespace formatting must match CAIP-2: {0}")]
    UnsupportedNamespaceKey(String),
}

impl ProposeNamespaceError {
    pub fn error_code(&self) -> i32 {
        match self {
            Self::UnsupportedChains(..)
            | Self::UnsupportedChainsEmpty
            | Self::UnsupportedChainsCaip2(..)
            | Self::UnsupportedChainsNamespace(..) => 5100,
            Self::UnsupportedEvents(..)
            | Self::UnsupportedExtensions
            | Self::UnsupportedMethods(..)
            | Self::UnsupportedNamespace(..)
            | Self::UnsupportedNamespaceKey(..) => 5104,
        }
    }
}

/// https://specs.walletconnect.com/2.0/specs/clients/sign/namespaces
#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProposeNamespaces(pub BTreeMap<String, ProposeNamespace>);

impl Deref for ProposeNamespaces {
    type Target = BTreeMap<String, ProposeNamespace>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ProposeNamespaces {
    /// Ensures that application is compatible with the requester requirements.
    ///
    /// Implementation must support at least all the elements in `required`.
    pub fn supported(&self, required: &ProposeNamespaces) -> Result<(), ProposeNamespaceError> {
        if self.is_empty() {
            return Err(ProposeNamespaceError::UnsupportedNamespace(
                "None supported".to_string(),
            ));
        }

        if required.is_empty() {
            return Ok(());
        }

        for (name, other) in required.iter() {
            let ours = self
                .get(name)
                .ok_or_else(|| ProposeNamespaceError::UnsupportedNamespace(name.to_string()))?;
            ours.supported(other)?;
        }

        Ok(())
    }

    pub fn caip2_validate(&self) -> Result<(), ProposeNamespaceError> {
        let caip_regex = get_caip2_regex();
        for (name, namespace) in self.iter() {
            let captures = caip_regex
                .captures(name)
                .ok_or_else(|| ProposeNamespaceError::UnsupportedNamespaceKey(name.to_string()))?;

            let name = captures
                .name("namespace")
                .expect("namespace name missing: unexpected error")
                .as_str();

            let reference = captures.name("reference").map(|m| m.as_str());

            namespace.chains_caip2_validate(name, reference)?;
        }

        Ok(())
    }
}

/// https://specs.walletconnect.com/2.0/specs/clients/sign/namespaces#
/// proposal-namespace
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProposeNamespace {
    pub chains: BTreeSet<String>,
    pub methods: BTreeSet<String>,
    pub events: BTreeSet<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub extensions: Option<Vec<Self>>,
}

impl ProposeNamespace {
    /// Ensures that application is compatible with the requester requirements.
    ///
    /// Implementation must support at least all the elements in `required`.
    pub fn supported(&self, required: &ProposeNamespace) -> Result<(), ProposeNamespaceError> {
        let join_error_elements =
            |required: &BTreeSet<String>, ours: &BTreeSet<String>| -> String {
                required
                    .difference(ours)
                    .map(|s| s.as_str())
                    .collect::<Vec<&str>>()
                    .join(",")
            };

        if !self.chains.is_superset(&required.chains) {
            return Err(ProposeNamespaceError::UnsupportedChains(
                join_error_elements(&required.chains, &self.chains),
            ));
        }

        if !self.methods.is_superset(&required.methods) {
            return Err(ProposeNamespaceError::UnsupportedMethods(
                join_error_elements(&required.methods, &self.methods),
            ));
        }

        if !self.events.is_superset(&required.events) {
            return Err(ProposeNamespaceError::UnsupportedEvents(
                join_error_elements(&required.events, &self.events),
            ));
        }

        // TODO: extension comparison probably should follow the same
        // validation process as above for non optional namespaces.
        match (&self.extensions, &required.extensions) {
            (Some(this), Some(other)) => {
                if !other.iter().all(|item| this.contains(item)) {
                    return Err(ProposeNamespaceError::UnsupportedExtensions);
                }
            }
            (Some(_), None) => {
                return Err(ProposeNamespaceError::UnsupportedExtensions);
            }
            (None, Some(_)) | (None, None) => {}
        }

        Ok(())
    }

    pub fn chains_caip2_validate(
        &self,
        namespace: &str,
        reference: Option<&str>,
    ) -> Result<(), ProposeNamespaceError> {
        println!("{namespace}, {reference:?}");

        // https://specs.walletconnect.com/2.0/specs/clients/sign/
        // namespaces#13-chains-might-be-omitted-if-the-caip-2-is-defined-in-the-index
        match (reference, self.chains.is_empty()) {
            (None, true) => return Err(ProposeNamespaceError::UnsupportedChainsEmpty),
            (Some(_), true) => return Ok(()),
            _ => {}
        }

        let caip_regex = get_caip2_regex();
        for chain in self.chains.iter() {
            let captures = caip_regex
                .captures(chain)
                .ok_or_else(|| ProposeNamespaceError::UnsupportedChainsCaip2(chain.to_string()))?;

            let chain_namespace = captures
                .name("namespace")
                .expect("chain namespace name is missing: unexpected error")
                .as_str();

            if namespace != chain_namespace {
                return Err(ProposeNamespaceError::UnsupportedChainsNamespace(
                    namespace.to_string(),
                    chain_namespace.to_string(),
                ));
            }

            let chain_reference =
                captures
                    .name("reference")
                    .map(|m| m.as_str())
                    .ok_or_else(|| {
                        ProposeNamespaceError::UnsupportedChainsCaip2(namespace.to_string())
                    })?;

            if let Some(r) = reference {
                if r != chain_reference {
                    return Err(ProposeNamespaceError::UnsupportedChainsCaip2(
                        namespace.to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================================================
    // https://specs.walletconnect.com/2.0/specs/clients/sign/namespaces#
    // rejecting-a-session-response
    // - validates namespaces match at least all requiredNamespaces
    // ========================================================================================================

    fn test_namespace() -> ProposeNamespace {
        let test_vec = vec![
            "0".to_string(),
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
        ];
        ProposeNamespace {
            chains: BTreeSet::from_iter(test_vec.clone()),
            methods: BTreeSet::from_iter(test_vec.clone()),
            events: BTreeSet::from_iter(test_vec.clone()),
            extensions: None,
        }
    }

    /// https://specs.walletconnect.com/2.0/specs/clients/sign/namespaces#
    /// 19-proposal-namespaces-may-be-empty
    #[test]
    fn namespaces_required_empty_success() {
        let namespaces = ProposeNamespaces({
            let mut map: BTreeMap<String, ProposeNamespace> = BTreeMap::new();
            map.insert(
                "1".to_string(),
                ProposeNamespace {
                    ..Default::default()
                },
            );
            map
        });
        assert!(namespaces
            .supported(&ProposeNamespaces(
                BTreeMap::<String, ProposeNamespace>::new()
            ))
            .is_ok())
    }

    #[test]
    fn namespace_unsupported_chains_failure() {
        let theirs = test_namespace();
        let mut ours = test_namespace();

        ours.chains.remove("1");
        assert_eq!(
            ours.supported(&theirs),
            Err(ProposeNamespaceError::UnsupportedChains("1".to_string())),
        );

        ours.chains.remove("2");
        assert_eq!(
            ours.supported(&theirs),
            Err(ProposeNamespaceError::UnsupportedChains("1,2".to_string())),
        );
    }

    #[test]
    fn namespace_unsupported_methods_failure() {
        let theirs = test_namespace();
        let mut ours = test_namespace();

        ours.methods.remove("1");
        assert_eq!(
            ours.supported(&theirs),
            Err(ProposeNamespaceError::UnsupportedMethods("1".to_string())),
        );

        ours.methods.remove("2");
        assert_eq!(
            ours.supported(&theirs),
            Err(ProposeNamespaceError::UnsupportedMethods("1,2".to_string())),
        );
    }

    #[test]
    fn namespace_unsupported_events_failure() {
        let theirs = test_namespace();
        let mut ours = test_namespace();

        ours.events.remove("1");
        assert_eq!(
            ours.supported(&theirs),
            Err(ProposeNamespaceError::UnsupportedEvents("1".to_string())),
        );

        ours.events.remove("2");
        assert_eq!(
            ours.supported(&theirs),
            Err(ProposeNamespaceError::UnsupportedEvents("1,2".to_string())),
        );
    }

    // ========================================================================================================
    // CAIP-2 TESTS: https://chainagnostic.org/CAIPs/caip-2
    // ========================================================================================================
    #[test]
    fn caip2_test_cases() -> Result<(), ProposeNamespaceError> {
        let chains = [
            // Ethereum mainnet
            "eip155:1",
            // Bitcoin mainnet (see https://github.com/bitcoin/bips/blob/master/bip-0122.mediawiki#definition-of-chain-id)
            "bip122:000000000019d6689c085ae165831e93",
            // Litecoin
            "bip122:12a765e31ffd4059bada1e25190f6e98",
            // Feathercoin (Litecoin fork)
            "bip122:fdbe99b90c90bae7505796461471d89a",
            // Cosmos Hub (Tendermint + Cosmos SDK)
            "cosmos:cosmoshub-2",
            "cosmos:cosmoshub-3",
            // Binance chain (Tendermint + Cosmos SDK; see https://dataseed5.defibit.io/genesis)
            "cosmos:Binance-Chain-Tigris",
            // IOV Mainnet (Tendermint + weave)
            "cosmos:iov-mainnet",
            // StarkNet Testnet
            "starknet:SN_GOERLI",
            // Lisk Mainnet (LIP-0009; see https://github.com/LiskHQ/lips/blob/master/proposals/lip-0009.md)
            "lip9:9ee11e9df416b18b",
            // Dummy max length (8+1+32 = 41 chars/bytes)
            "chainstd:8c3444cf8970a9e41a706fab93e7a6c4",
        ];

        let caip2_regex = get_caip2_regex();
        for chain in chains {
            caip2_regex
                .captures(chain)
                .ok_or_else(|| ProposeNamespaceError::UnsupportedChainsCaip2(chain.to_string()))?;
        }

        Ok(())
    }

    /// https://specs.walletconnect.com/2.0/specs/clients/sign/namespaces#
    /// 12-proposal-namespaces-must-not-have-chains-empty
    #[test]
    fn caip2_12_chains_empty_failure() {
        let namespaces = ProposeNamespaces({
            let mut map: BTreeMap<String, ProposeNamespace> = BTreeMap::new();
            map.insert(
                "eip155".to_string(),
                ProposeNamespace {
                    ..Default::default()
                },
            );
            map
        });

        assert_eq!(
            namespaces.caip2_validate(),
            Err(ProposeNamespaceError::UnsupportedChainsEmpty),
        );
    }

    /// https://specs.walletconnect.com/2.0/specs/clients/sign/namespaces#
    /// 13-chains-might-be-omitted-if-the-caip-2-is-defined-in-the-index
    #[test]
    fn caip2_13_chains_omitted_success() -> Result<(), ProposeNamespaceError> {
        let namespaces = ProposeNamespaces({
            let mut map: BTreeMap<String, ProposeNamespace> = BTreeMap::new();
            map.insert(
                "eip155:1".to_string(),
                ProposeNamespace {
                    ..Default::default()
                },
            );
            map
        });

        namespaces.caip2_validate()?;

        Ok(())
    }

    /// https://specs.walletconnect.com/2.0/specs/clients/sign/namespaces#
    /// 14-chains-must-be-caip-2-compliant
    #[test]
    fn caip2_14_must_be_compliant_failure() -> Result<(), ProposeNamespaceError> {
        let namespaces = ProposeNamespaces({
            let mut map: BTreeMap<String, ProposeNamespace> = BTreeMap::new();
            map.insert(
                "eip155".to_string(),
                ProposeNamespace {
                    chains: BTreeSet::from_iter(vec!["1".to_string()]),
                    ..Default::default()
                },
            );
            map
        });

        assert_eq!(
            namespaces.caip2_validate(),
            Err(ProposeNamespaceError::UnsupportedChainsCaip2(
                "1".to_string()
            )),
        );

        Ok(())
    }

    /// https://specs.walletconnect.com/2.0/specs/clients/sign/namespaces#
    /// 16-all-chains-in-the-namespace-must-contain-the-namespace-prefix
    #[test]
    fn caip2_16_chain_prefix_success() -> Result<(), ProposeNamespaceError> {
        let namespaces = ProposeNamespaces({
            let mut map: BTreeMap<String, ProposeNamespace> = BTreeMap::new();
            map.insert(
                "eip155".to_string(),
                ProposeNamespace {
                    chains: BTreeSet::from_iter(vec!["eip155:1".to_string()]),
                    ..Default::default()
                },
            );
            map.insert(
                "bip122".to_string(),
                ProposeNamespace {
                    chains: BTreeSet::from_iter(vec![
                        "bip122:000000000019d6689c085ae165831e93".to_string(),
                        "bip122:12a765e31ffd4059bada1e25190f6e98".to_string(),
                    ]),
                    ..Default::default()
                },
            );
            map.insert(
                "cosmos".to_string(),
                ProposeNamespace {
                    chains: BTreeSet::from_iter(vec![
                        "cosmos:cosmoshub-2".to_string(),
                        "cosmos:cosmoshub-3".to_string(),
                        "cosmos:Binance-Chain-Tigris".to_string(),
                        "cosmos:iov-mainnet".to_string(),
                    ]),
                    ..Default::default()
                },
            );
            map.insert(
                "starknet".to_string(),
                ProposeNamespace {
                    chains: BTreeSet::from_iter(vec!["starknet:SN_GOERLI".to_string()]),
                    ..Default::default()
                },
            );
            map.insert(
                "chainstd".to_string(),
                ProposeNamespace {
                    chains: BTreeSet::from_iter(vec![
                        "chainstd:8c3444cf8970a9e41a706fab93e7a6c4".to_string()
                    ]),
                    ..Default::default()
                },
            );
            map
        });

        namespaces.caip2_validate()?;

        Ok(())
    }

    /// https://specs.walletconnect.com/2.0/specs/clients/sign/namespaces#
    /// 16-all-chains-in-the-namespace-must-contain-the-namespace-prefix
    #[test]
    fn caip2_16_chain_prefix_failure() -> Result<(), ProposeNamespaceError> {
        let namespaces = ProposeNamespaces({
            let mut map: BTreeMap<String, ProposeNamespace> = BTreeMap::new();
            map.insert(
                "eip155".to_string(),
                ProposeNamespace {
                    chains: BTreeSet::from_iter(vec!["cosmos:1".to_string()]),
                    ..Default::default()
                },
            );
            map
        });

        assert_eq!(
            namespaces.caip2_validate(),
            Err(ProposeNamespaceError::UnsupportedChainsNamespace(
                "eip155".to_string(),
                "cosmos".to_string()
            )),
        );

        Ok(())
    }

    /// https://specs.walletconnect.com/2.0/specs/clients/sign/namespaces#
    /// 17-namespace-key-must-comply-with-caip-2-specification
    #[test]
    fn caip2_17_namespace_key_failure() -> Result<(), ProposeNamespaceError> {
        let namespaces = ProposeNamespaces({
            let mut map: BTreeMap<String, ProposeNamespace> = BTreeMap::new();
            map.insert(
                "".to_string(),
                ProposeNamespace {
                    chains: BTreeSet::from_iter(vec![":1".to_string()]),
                    ..Default::default()
                },
            );
            map
        });

        assert_eq!(
            namespaces.caip2_validate(),
            Err(ProposeNamespaceError::UnsupportedNamespaceKey(
                "".to_string()
            )),
        );

        let namespaces = ProposeNamespaces({
            let mut map: BTreeMap<String, ProposeNamespace> = BTreeMap::new();
            map.insert(
                "**".to_string(),
                ProposeNamespace {
                    chains: BTreeSet::from_iter(vec!["**:1".to_string()]),
                    ..Default::default()
                },
            );
            map
        });

        assert_eq!(
            namespaces.caip2_validate(),
            Err(ProposeNamespaceError::UnsupportedNamespaceKey(
                "**".to_string()
            )),
        );

        Ok(())
    }
}
