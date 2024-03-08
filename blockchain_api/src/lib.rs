pub use reqwest::Error;
use {
    relay_rpc::{auth::cacao::signature::eip1271::get_rpc_url::GetRpcUrl, domain::ProjectId},
    serde::Deserialize,
    std::{collections::HashSet, convert::Infallible, sync::Arc, time::Duration},
    tokio::{sync::RwLock, task::JoinHandle},
    tracing::error,
    url::Url,
};

const BLOCKCHAIN_API_SUPPORTED_CHAINS_ENDPOINT_STR: &str = "/v1/supported-chains";
const BLOCKCHAIN_API_RPC_ENDPOINT_STR: &str = "/v1";
const BLOCKCHAIN_API_RPC_CHAIN_ID_PARAM: &str = "chainId";
const BLOCKCHAIN_API_RPC_PROJECT_ID_PARAM: &str = "projectId";

const SUPPORTED_CHAINS_REFRESH_INTERVAL: Duration = Duration::from_secs(60 * 60 * 4);

#[derive(Debug, Deserialize)]
struct SupportedChainsResponse {
    pub http: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct BlockchainApiProvider {
    project_id: ProjectId,
    blockchain_api_rpc_endpoint: Url,
    supported_chains: Arc<RwLock<HashSet<String>>>,
    refresh_job: Arc<JoinHandle<Infallible>>,
}

impl Drop for BlockchainApiProvider {
    fn drop(&mut self) {
        self.refresh_job.abort();
    }
}

async fn refresh_supported_chains(
    blockchain_api_supported_chains_endpoint: Url,
    supported_chains: &RwLock<HashSet<String>>,
) -> Result<(), Error> {
    let response = reqwest::get(blockchain_api_supported_chains_endpoint)
        .await?
        .json::<SupportedChainsResponse>()
        .await?;
    *supported_chains.write().await = response.http;
    Ok(())
}

impl BlockchainApiProvider {
    pub async fn new(project_id: ProjectId, blockchain_api_endpoint: Url) -> Result<Self, Error> {
        let blockchain_api_rpc_endpoint = blockchain_api_endpoint
            .join(BLOCKCHAIN_API_RPC_ENDPOINT_STR)
            .expect("Safe unwrap: hardcoded URL: BLOCKCHAIN_API_RPC_ENDPOINT_STR");
        let blockchain_api_supported_chains_endpoint = blockchain_api_endpoint
            .join(BLOCKCHAIN_API_SUPPORTED_CHAINS_ENDPOINT_STR)
            .expect("Safe unwrap: hardcoded URL: BLOCKCHAIN_API_SUPPORTED_CHAINS_ENDPOINT_STR");

        let supported_chains = Arc::new(RwLock::new(HashSet::new()));
        refresh_supported_chains(
            blockchain_api_supported_chains_endpoint.clone(),
            &supported_chains,
        )
        .await?;
        let mut interval = tokio::time::interval(SUPPORTED_CHAINS_REFRESH_INTERVAL);
        interval.tick().await;
        let refresh_job = tokio::task::spawn({
            let supported_chains = supported_chains.clone();
            let blockchain_api_supported_chains_endpoint =
                blockchain_api_supported_chains_endpoint.clone();
            async move {
                loop {
                    interval.tick().await;
                    if let Err(e) = refresh_supported_chains(
                        blockchain_api_supported_chains_endpoint.clone(),
                        &supported_chains,
                    )
                    .await
                    {
                        error!("Failed to refresh supported chains: {e}");
                    }
                }
            }
        });
        Ok(Self {
            project_id,
            blockchain_api_rpc_endpoint,
            supported_chains,
            refresh_job: Arc::new(refresh_job),
        })
    }
}

fn build_rpc_url(blockchain_api_rpc_endpoint: Url, chain_id: &str, project_id: &str) -> Url {
    let mut url = blockchain_api_rpc_endpoint;
    url.query_pairs_mut()
        .append_pair(BLOCKCHAIN_API_RPC_CHAIN_ID_PARAM, chain_id)
        .append_pair(BLOCKCHAIN_API_RPC_PROJECT_ID_PARAM, project_id);
    url
}

impl GetRpcUrl for BlockchainApiProvider {
    async fn get_rpc_url(&self, chain_id: String) -> Option<Url> {
        self.supported_chains
            .read()
            .await
            .contains(&chain_id)
            .then(|| {
                build_rpc_url(
                    self.blockchain_api_rpc_endpoint.clone(),
                    &chain_id,
                    self.project_id.as_ref(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rpc_endpoint() {
        assert_eq!(
            build_rpc_url(
                "https://rpc.walletconnect.com/v1".parse().unwrap(),
                "eip155:1",
                "my-project-id"
            )
            .as_str(),
            "https://rpc.walletconnect.com/v1?chainId=eip155%3A1&projectId=my-project-id"
        );
    }
}
