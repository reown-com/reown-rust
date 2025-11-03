use {
    futures_util::TryStreamExt as _,
    relay_client::{
        error::ClientError,
        websocket::{Client, CloseFrame, ConnectionHandler, PublishedMessage},
        ConnectionOptions,
    },
    relay_rpc::{
        auth::{ed25519_dalek::SigningKey, AuthToken},
        domain::Topic,
    },
    std::time::Duration,
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

    fn disconnected(&mut self, frame: Option<CloseFrame>) {
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

    let app_client = Client::new(Handler::new("client1"));
    app_client
        .connect(&create_conn_opts(&args.address, &args.project_id))
        .await?;

    let wallet_client = Client::new(Handler::new("client2"));
    wallet_client
        .connect(&create_conn_opts(&args.address, &args.project_id))
        .await?;

    // Pre-generate topics, while the actual clients would derive them from the keys
    // exchanged during pairing:
    let pairing_topic = Topic::generate();
    let session_topic = Topic::generate();

    // App proposes session:
    app_client
        .propose_session(
            pairing_topic.clone(),
            "wc_sessionPropose_req",
            Some("attestation".into()),
            None,
        )
        .await?;
    println!("[client1] proposed session: pairing_topic={pairing_topic}");

    // Wallet scans the QR code and receives the `wc_sessionPropose` request:
    let msg = wallet_client
        .fetch_stream([pairing_topic.clone()])
        .try_collect::<Vec<_>>()
        .await?
        .pop()
        .unwrap();
    println!("[client2] received session proposal: {msg:?}");

    // After user confirmation, the wallet approves this session:
    wallet_client
        .approve_session(
            pairing_topic.clone(),
            session_topic.clone(),
            "wc_sessionPropose_res",
            "wc_sessionSettle_req",
            Default::default(),
            None,
        )
        .await?;
    println!(
        "[client2] approved session: pairing_topic={pairing_topic} session_topic={session_topic}"
    );

    // App receives `wc_sessionPropose` response, derives `session_topic` and
    // subscribes to it:
    app_client.subscribe(session_topic.clone()).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    // App responds to the `wc_sessionSettle`:
    app_client
        .publish(
            session_topic.clone(),
            "wc_sessionSettle_res",
            None,
            1103,
            Duration::from_secs(300),
            false,
        )
        .await?;
    println!("[client1] published `wc_sessionSettle` response: session_topic={session_topic}");

    tokio::time::sleep(Duration::from_millis(1000)).await;

    drop(app_client);
    drop(wallet_client);

    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("clients disconnected");

    Ok(())
}
