//! https://specs.walletconnect.com/2.0/specs/clients/sign/data-structures

mod propose_namespaces;
mod settle_namespaces;

use serde::{Deserialize, Serialize};
pub use {
    propose_namespaces::{ProposeNamespace, ProposeNamespaceError, ProposeNamespaces},
    settle_namespaces::{SettleNamespace, SettleNamespaces},
};

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub description: String,
    pub url: String,
    pub icons: Vec<String>,
    pub name: String,
}

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone, Default)]
pub struct Relay {
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub data: Option<String>,
}
