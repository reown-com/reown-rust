use {alloy_provider::Provider, alloy_transport::Transport};

pub trait GetProvider {
    type Transport: Transport + Clone;
    type Provider: Provider<Self::Transport>;

    #[allow(async_fn_in_trait)]
    async fn get_provider(&self, chain_id: String) -> Option<Self::Provider>;
}
