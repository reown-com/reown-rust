use {
    relay_client::{
        http::{Client, WatchRegisterRequest},
        ConnectionOptions,
    },
    relay_rpc::{
        auth::{ed25519_dalek::SigningKey, AuthToken},
        domain::{DecodedClientId, Topic},
        jwt::VerifyableClaims,
        rpc,
    },
    std::{
        net::{Ipv4Addr, SocketAddr, SocketAddrV4},
        sync::Arc,
        time::Duration,
    },
    structopt::StructOpt,
    tokio::{sync::mpsc, task::JoinHandle},
    url::Url,
    warp::Filter,
};

#[derive(StructOpt)]
struct Args {
    /// Specify HTTP address.
    #[structopt(short, long, default_value = "https://relay.walletconnect.com/rpc")]
    address: String,

    /// Specify WalletConnect project ID.
    #[structopt(short, long, default_value = "3cbaa32f8fbf3cdcc87d27ca1fa68069")]
    project_id: String,

    /// Webhook server port.
    #[structopt(short, long, default_value = "10100")]
    webhook_server_port: u16,
}

fn create_conn_opts(key: &SigningKey, address: &str, project_id: &str) -> ConnectionOptions {
    let aud = Url::parse(address)
        .unwrap()
        .origin()
        .unicode_serialization();

    let auth = AuthToken::new("http://example.com")
        .aud(aud)
        .ttl(Duration::from_secs(60 * 60))
        .as_jwt(key)
        .unwrap();

    ConnectionOptions::new(project_id, auth).with_address(address)
}

#[derive(Debug)]
pub struct WebhookData {
    pub url: String,
    pub payload: rpc::WatchWebhookPayload,
}

pub struct WebhookServer {
    addr: SocketAddr,
    handle: JoinHandle<()>,
    payload_rx: mpsc::UnboundedReceiver<WebhookData>,
}

impl WebhookServer {
    pub fn new(port: u16) -> Self {
        let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port).into();
        let (payload_tx, payload_rx) = mpsc::unbounded_channel();
        let handle = tokio::spawn(mock_webhook_server(addr, payload_tx));

        Self {
            addr,
            handle,
            payload_rx,
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub async fn recv(&mut self) -> WebhookData {
        self.payload_rx.recv().await.unwrap()
    }
}

impl Drop for WebhookServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn mock_webhook_server(addr: SocketAddr, payload_tx: mpsc::UnboundedSender<WebhookData>) {
    let routes = warp::post()
        .and(warp::path::tail())
        .and(warp::body::json())
        .and(warp::any().map(move || payload_tx.clone()))
        .then(
            move |path: warp::path::Tail,
                  payload: rpc::WatchWebhookPayload,
                  payload_tx: mpsc::UnboundedSender<WebhookData>| async move {
                let url = format!("http://{addr}/{}", path.as_str());
                payload_tx.send(WebhookData { url, payload }).unwrap();
                warp::reply()
            },
        );

    warp::serve(routes).run(addr).await;
}

/// Note: This example will only work with a locally running relay, since it
/// requires access to the local HTTP server.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    const PUB_WH_PATH: &str = "/publisher_webhook";
    const SUB_WH_PATH: &str = "/subscriber_webhook";

    let args = Args::from_args();
    let mut server = WebhookServer::new(args.webhook_server_port);
    let server_url = server.url();

    // Give time for the server to start up.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let publisher_key = SigningKey::generate(&mut rand::thread_rng());
    let publisher = Client::new(&create_conn_opts(
        &publisher_key,
        &args.address,
        &args.project_id,
    ))?;
    println!(
        "[publisher] client id: {}",
        DecodedClientId::from(publisher_key.verifying_key()).to_did_key()
    );

    let subscriber_key = SigningKey::generate(&mut rand::thread_rng());
    let subscriber = Client::new(&create_conn_opts(
        &subscriber_key,
        &args.address,
        &args.project_id,
    ))?;
    println!(
        "[subscriber] client id: {}",
        DecodedClientId::from(subscriber_key.verifying_key()).to_did_key()
    );

    let topic = Topic::generate();
    let message: Arc<str> = Arc::from("Hello WalletConnect!");

    let sub_relay_id: DecodedClientId = subscriber
        .watch_register(
            WatchRegisterRequest {
                service_url: server_url.clone(),
                webhook_url: format!("{}{}", server_url, SUB_WH_PATH),
                watch_type: rpc::WatchType::Subscriber,
                tags: vec![1100],
                statuses: vec![rpc::WatchStatus::Queued],
                ttl: Duration::from_secs(600),
            },
            &subscriber_key,
        )
        .await
        .unwrap()
        .relay_id
        .into();
    subscriber.subscribe(topic.clone()).await.unwrap();
    println!(
        "[subscriber] watch registered: relay_id={}",
        sub_relay_id.to_did_key()
    );

    let pub_relay_id: DecodedClientId = publisher
        .watch_register(
            WatchRegisterRequest {
                service_url: server_url.clone(),
                webhook_url: format!("{}{}", server_url, PUB_WH_PATH),
                watch_type: rpc::WatchType::Publisher,
                tags: vec![1100],
                statuses: vec![rpc::WatchStatus::Accepted],
                ttl: Duration::from_secs(600),
            },
            &publisher_key,
        )
        .await
        .unwrap()
        .relay_id
        .into();
    println!(
        "[publisher] watch registered: relay_id={}",
        pub_relay_id.to_did_key()
    );

    publisher
        .publish(
            topic.clone(),
            message.clone(),
            None,
            1100,
            Duration::from_secs(30),
            false,
        )
        .await
        .unwrap();
    println!("[publisher] message published: topic={topic} message={message}");

    tokio::time::sleep(Duration::from_secs(1)).await;

    let messages = subscriber.fetch(topic).await?.messages;
    let message = messages
        .first()
        .ok_or(anyhow::anyhow!("fetch did not return any messages"))?;
    println!("[subscriber] received message: {}", message.message);

    let pub_data = server.recv().await;
    let decoded =
        rpc::WatchEventClaims::try_from_str(pub_data.payload.event_auth.first().unwrap()).unwrap();
    let decoded_json = serde_json::to_string_pretty(&decoded).unwrap();
    println!(
        "[webhook] publisher: url={} data={}",
        pub_data.url, decoded_json
    );

    let sub_data = server.recv().await;
    let decoded =
        rpc::WatchEventClaims::try_from_str(sub_data.payload.event_auth.first().unwrap()).unwrap();
    let decoded_json = serde_json::to_string_pretty(&decoded).unwrap();
    println!(
        "[webhook] subscriber: url={} data={}",
        sub_data.url, decoded_json
    );

    Ok(())
}
