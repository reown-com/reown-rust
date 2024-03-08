use url::Url;

pub trait GetRpcUrl {
    #[allow(async_fn_in_trait)]
    async fn get_rpc_url(&self, chain_id: String) -> Option<Url>;
}
