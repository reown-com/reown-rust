use {
    relay_client::{
        error::ClientError,
        websocket::{Client, CloseFrame, ConnectionHandler, PublishedMessage},
        ConnectionOptions,
    },
    relay_rpc::{
        auth::{ed25519_dalek::SigningKey, AuthToken},
        domain::Topic,
    },
    std::{sync::Arc, time::Duration},
    structopt::StructOpt,
};

#[derive(StructOpt)]
struct Args {
    /// Specify WebSocket address.
    #[structopt(short, long, default_value = "wss://relay.walletconnect.com")]
    address: String,

    /// Specify WalletConnect project ID.
    #[structopt(short, long, default_value = "3cbaa32f8fbf3cdcc87d27ca1fa68069")]
    project_id: String,
}

struct Handler {
    name: &'static str,
}

impl Handler {
    fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl ConnectionHandler for Handler {
    fn connected(&mut self) {
        println!("[{}] connection open", self.name);
    }

    fn disconnected(&mut self, frame: Option<CloseFrame<'static>>) {
        println!("[{}] connection closed: frame={frame:?}", self.name);
    }

    fn message_received(&mut self, message: PublishedMessage) {
        println!(
            "[{}] inbound message: topic={} message={}",
            self.name, message.topic, message.message
        );
    }

    fn inbound_error(&mut self, error: ClientError) {
        println!("[{}] inbound error: {error}", self.name);
    }

    fn outbound_error(&mut self, error: ClientError) {
        println!("[{}] outbound error: {error}", self.name);
    }
}

fn create_conn_opts(address: &str, project_id: &str) -> ConnectionOptions {
    let key = SigningKey::generate(&mut rand::thread_rng());

    let auth = AuthToken::new("http://example.com")
        .aud(address)
        .ttl(Duration::from_secs(60 * 60))
        .as_jwt(&key)
        .unwrap();

    ConnectionOptions::new(project_id, auth).with_address(address)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::from_args();

    let client1 = Client::new(Handler::new("client1"));
    client1
        .connect(&create_conn_opts(&args.address, &args.project_id))
        .await?;

    let client2 = Client::new(Handler::new("client2"));
    client2
        .connect(&create_conn_opts(&args.address, &args.project_id))
        .await?;

    let topic = Topic::generate();

    let subscription_id = client1.subscribe(topic.clone()).await?;
    println!("[client1] subscribed: topic={topic} subscription_id={subscription_id}");

    client2
        .publish(
            topic.clone(),
            Arc::from("Hello WalletConnect!"),
            None,
            0,
            Duration::from_secs(60),
            false,
        )
        .await?;

    println!("[client2] published message with topic: {topic}",);

    tokio::time::sleep(Duration::from_millis(500)).await;

    drop(client1);
    drop(client2);

    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("clients disconnected");

    Ok(())
}
