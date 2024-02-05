use {super::get_rpc_url::GetRpcUrl, crate::domain::ProjectId, url::Url};

// https://github.com/WalletConnect/blockchain-api/blob/master/SUPPORTED_CHAINS.md
const SUPPORTED_CHAINS: [&str; 26] = [
    "eip155:1",
    "eip155:5",
    "eip155:11155111",
    "eip155:10",
    "eip155:420",
    "eip155:42161",
    "eip155:421613",
    "eip155:137",
    "eip155:80001",
    "eip155:1101",
    "eip155:42220",
    "eip155:1313161554",
    "eip155:1313161555",
    "eip155:56",
    "eip155:56",
    "eip155:43114",
    "eip155:43113",
    "eip155:324",
    "eip155:280",
    "near",
    "eip155:100",
    "solana:4sgjmw1sunhzsxgspuhpqldx6wiyjntz",
    "eip155:8453",
    "eip155:84531",
    "eip155:7777777",
    "eip155:999",
];

#[derive(Debug, Clone)]
pub struct BlockchainApiProvider {
    project_id: ProjectId,
}

impl BlockchainApiProvider {
    pub fn new(project_id: ProjectId) -> Self {
        Self { project_id }
    }
}

impl GetRpcUrl for BlockchainApiProvider {
    fn get_rpc_url(&self, chain_id: String) -> Option<Url> {
        if SUPPORTED_CHAINS.contains(&chain_id.as_str()) {
            Some(
                format!(
                    "https://rpc.walletconnect.com/v1?chainId={chain_id}&projectId={}",
                    self.project_id
                )
                .parse()
                .expect("Provider URL should be valid"),
            )
        } else {
            None
        }
    }
}
