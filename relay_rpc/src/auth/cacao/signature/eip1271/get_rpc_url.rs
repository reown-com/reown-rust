use url::Url;

pub trait GetRpcUrl {
    fn get_rpc_url(&self, chain_id: String) -> Option<Url>;
}
