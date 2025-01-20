mod utils;

use {
    rand::rngs::OsRng,
    std::{
        fmt::{Display, Formatter},
        time::Duration,
    },
    walletconnect_sdk::{
        client::{
            error::ClientError,
            websocket::{Client, CloseFrame, ConnectionHandler, PublishedMessage},
            ConnectionOptions,
        },
        rpc::{
            auth::{ed25519_dalek::SigningKey, AuthToken},
            domain::Topic,
        },
    },
    wasm_bindgen::prelude::*,
    wasm_bindgen_futures::spawn_local,
    web_sys::console,
};

enum ClientId {
    WC1,
    WC2,
}

impl ClientId {
    fn div(&self, d: &str) -> String {
        format!("{}{d}", self)
    }
}
impl Display for ClientId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ClientId::WC1 => write!(f, "wc1"),
            ClientId::WC2 => write!(f, "wc2"),
        }
    }
}

struct Handler {
    client_id: ClientId,
}

impl Handler {
    fn new(name: ClientId) -> Self {
        Self { client_id: name }
    }

    fn error_div(&self) -> String {
        self.client_id.div("error")
    }

    fn connect_div(&self) -> String {
        self.client_id.div("connect")
    }

    fn message_div(&self) -> String {
        self.client_id.div("message")
    }
}

impl ConnectionHandler for Handler {
    fn connected(&mut self) {
        let _ = utils::set_result_text(&self.connect_div(), "connected");
    }

    fn disconnected(&mut self, _frame: Option<CloseFrame<'static>>) {
        let _ = utils::set_result_text(&self.connect_div(), "disconnected");
    }

    fn message_received(&mut self, message: PublishedMessage) {
        let div = self.message_div();
        let from = match self.client_id {
            ClientId::WC1 => "wc2",
            ClientId::WC2 => "wc1",
        };
        let msg = format!("message from {from}: '{}'", message.message.as_ref());
        let _ = utils::set_result_text(&div, &msg);
    }

    fn inbound_error(&mut self, error: ClientError) {
        let e = format!("inbound error: {error}");
        let _ = utils::set_result_text(&self.error_div(), &e);
    }

    fn outbound_error(&mut self, error: ClientError) {
        let e = format!("outbound error: {error}");
        let _ = utils::set_result_text(&self.error_div(), &e);
    }
}

fn create_opts_result(address: &str, project_id: &str) -> anyhow::Result<ConnectionOptions> {
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    console::log_1(&"loaded key".into());
    let auth = AuthToken::new("http://example.com")
        .aud(address)
        .ttl(Duration::from_secs(60 * 60));
    console::log_1(&"AuthToken Init".into());
    let auth = auth.as_jwt(&key)?;
    console::log_1(&"AuthToken JWT".into());
    Ok(ConnectionOptions::new(project_id, auth).with_address(address))
}

fn create_conn_opts(address: &str, project_id: &str) -> Option<ConnectionOptions> {
    match create_opts_result(address, project_id) {
        Ok(o) => Some(o),
        Err(e) => {
            let error_msg = format!("Failed to create connection options: {:?}", e);
            let _ = utils::set_result_text(&ClientId::WC1.div("error"), &error_msg);
            let _ = utils::set_result_text(&ClientId::WC2.div("error"), &error_msg);
            None
        }
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    let project_id = env!("PROJECT_ID");
    spawn_local(async {
        let client1 = Client::new(Handler::new(ClientId::WC1));
        let client2 = Client::new(Handler::new(ClientId::WC2));
        let opts = create_conn_opts("wss://relay.walletconnect.org", project_id);
        if opts.is_none() {
            return;
        }
        utils::connect("wc1", &client1, &opts.unwrap()).await;
        let opts = create_conn_opts("wss://relay.walletconnect.org", project_id);
        if opts.is_none() {
            return;
        }
        utils::connect("wc2", &client2, &opts.unwrap()).await;

        let topic = Topic::generate();
        let sub = utils::subscribe_topic(ClientId::WC1, &client1, topic.clone()).await;
        if !sub {
            return;
        }
        let sub = utils::subscribe_topic(ClientId::WC2, &client2, topic.clone()).await;
        if !sub {
            return;
        }
        spawn_local(utils::publish(ClientId::WC1, client1, topic.clone()));
        spawn_local(utils::publish(ClientId::WC2, client2, topic.clone()));
        console::log_1(&"done".into());
    });
}
