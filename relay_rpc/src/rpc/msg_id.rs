use {
    crate::rpc,
    sha2::{Digest, Sha256},
    std::sync::Arc,
};

pub trait MsgId {
    fn msg_id(&self) -> Arc<str>;
}

impl MsgId for rpc::Publish {
    fn msg_id(&self) -> Arc<str> {
        let msg_id = Sha256::new()
            .chain_update(self.message.as_ref().as_bytes())
            .finalize();
        format!("{msg_id:x}").into()
    }
}

impl MsgId for rpc::Subscription {
    fn msg_id(&self) -> Arc<str> {
        let msg_id = Sha256::new()
            .chain_update(self.data.message.as_ref().as_bytes())
            .finalize();
        format!("{msg_id:x}").into()
    }
}
