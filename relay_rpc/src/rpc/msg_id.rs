use {
    crate::rpc,
    sha2::{Digest, Sha256},
};

pub trait MsgId {
    fn msg_id(&self) -> String;
}

impl MsgId for rpc::Publish {
    fn msg_id(&self) -> String {
        get_message_id(&self.message)
    }
}

impl MsgId for rpc::Subscription {
    fn msg_id(&self) -> String {
        get_message_id(&self.data.message)
    }
}

pub fn get_message_id(message: &str) -> String {
    let msg_id = Sha256::new().chain_update(message.as_bytes()).finalize();
    format!("{msg_id:x}")
}
