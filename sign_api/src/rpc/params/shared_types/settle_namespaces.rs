use {
    serde::{Deserialize, Serialize},
    std::{
        collections::{BTreeMap, BTreeSet},
        ops::Deref,
    },
};

/// TODO: some validation from `ProposeNamespaces` should be re-used.
/// TODO: caip-10 validation.
/// TODO: named errors like in `ProposeNamespaces`.
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettleNamespaces(pub BTreeMap<String, SettleNamespace>);

impl Deref for SettleNamespaces {
    type Target = BTreeMap<String, SettleNamespace>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettleNamespace {
    pub accounts: BTreeSet<String>,
    pub methods: BTreeSet<String>,
    pub events: BTreeSet<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub extensions: Option<Vec<Self>>,
}
