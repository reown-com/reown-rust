use {
    derive_more::Deref,
    future_timing::Timing,
    futures_concurrency::future::Join as _,
    futures_util::{FutureExt as _, TryStreamExt as _},
    relay_client::{
        ConnectionOptions,
        error::ClientError,
        websocket::{Client as NativeClient, CloseFrame, ConnectionHandler, PublishedMessage},
    },
    relay_rpc::{
        auth::{AuthToken, ed25519_dalek::SigningKey},
        domain::Topic,
    },
    std::time::{Duration, Instant},
    structopt::StructOpt,
    tokio::sync::mpsc,
};

mod log;

trait FutureTimingExt: Future + Sized {
    type Output;

    fn timed(self) -> impl Future<Output = <Self as FutureTimingExt>::Output>;
}

impl<T, U, E> FutureTimingExt for T
where
    T: Future<Output = Result<U, E>> + Sized,
{
    type Output = Result<(Timing, U), E>;

    fn timed(self) -> impl Future<Output = <Self as FutureTimingExt>::Output> {
        future_timing::timed(self).map(|out| {
            let (timing, res) = out.into_parts();
            res.map(|val| (timing, val))
        })
    }
}

trait TimingExt {
    fn total(&self) -> Duration;
}

impl TimingExt for Timing {
    fn total(&self) -> Duration {
        self.busy() + self.idle()
    }
}

#[derive(StructOpt)]
struct Args {
    /// Specify WebSocket address.
    #[structopt(short, long, default_value = "wss://relay.walletconnect.com")]
    address: String,

    /// Specify WalletConnect project ID.
    #[structopt(short, long)]
    project_id: String,
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

#[derive(Deref)]
struct Client {
    #[deref]
    client: NativeClient,
    msg_rx: mpsc::UnboundedReceiver<PublishedMessage>,
}

impl Client {
    async fn new(name: &'static str, address: &str, project_id: &str) -> anyhow::Result<Self> {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();

        let client = NativeClient::new(Handler::new(name, msg_tx));
        client
            .connect(&create_conn_opts(address, project_id))
            .await?;

        Ok(Self { client, msg_rx })
    }

    async fn receive(&mut self, topic: &Topic, message: &str) -> anyhow::Result<PublishedMessage> {
        loop {
            let msg = self.receive_next().await?;

            if &msg.topic == topic && msg.message.as_ref() == message {
                return Ok(msg);
            }
        }
    }

    async fn receive_next(&mut self) -> anyhow::Result<PublishedMessage> {
        let msg = tokio::time::timeout(Duration::from_secs(4), self.msg_rx.recv())
            .await?
            .ok_or_else(|| anyhow::anyhow!("message channel closed"))?;

        Ok(msg)
    }
}

struct Handler {
    msg_tx: mpsc::UnboundedSender<PublishedMessage>,
    name: &'static str,
}

impl Handler {
    fn new(name: &'static str, msg_tx: mpsc::UnboundedSender<PublishedMessage>) -> Self {
        Self { msg_tx, name }
    }
}

impl ConnectionHandler for Handler {
    fn connected(&mut self) {
        tracing::debug!(client = self.name, "connection open");
    }

    fn disconnected(&mut self, frame: Option<CloseFrame>) {
        tracing::debug!(client = self.name, ?frame, "connection closed");
    }

    fn message_received(&mut self, msg: PublishedMessage) {
        tracing::debug!(
            client = self.name,
            topic = %msg.topic,
            message = %msg.message,
            "inbound message"
        );

        let _ = self.msg_tx.send(msg);
    }

    fn inbound_error(&mut self, err: ClientError) {
        tracing::info!(client = self.name, ?err, "inbound error");
    }

    fn outbound_error(&mut self, err: ClientError) {
        tracing::info!(client = self.name, ?err, "outbound error");
    }
}

fn generate_message() -> String {
    Topic::generate().to_string()
}

#[derive(Debug)]
struct OneWayPing {
    subscribe: Duration,
    publish: Duration,
    receive: Duration,
}

async fn one_way_ping(mut c1: Client, c2: Client) -> anyhow::Result<OneWayPing> {
    let topic = Topic::generate();
    let msg = generate_message();
    let msg = msg.as_str();

    let (sub_time, _) = c1.subscribe(topic.clone()).timed().await?;

    let pub_fut = c2
        .publish(
            topic.clone(),
            msg,
            None,
            1100,
            Duration::from_secs(60),
            false,
        )
        .timed();

    let recv_fut = c1.receive(&topic, msg).timed();

    let (pub_res, recv_res) = (pub_fut, recv_fut).join().await;

    let (pub_time, _) = pub_res?;
    let (recv_time, _) = recv_res?;

    tokio::time::sleep(Duration::from_millis(150)).await;

    Ok(OneWayPing {
        subscribe: sub_time.total(),
        publish: pub_time.total(),
        receive: recv_time.total(),
    })
}

#[derive(Debug)]
struct TwoWayPing {
    subscribe1: Duration,
    subscribe2: Duration,
    publish1: Duration,
    publish2: Duration,
    receive1: Duration,
    receive2: Duration,
}

async fn two_way_ping(mut c1: Client, mut c2: Client) -> anyhow::Result<TwoWayPing> {
    let topic = Topic::generate();
    let msg = generate_message();
    let msg1 = msg.as_str();
    let msg = generate_message();
    let msg2 = msg.as_str();

    let (sub1_res, sub2_res) = (
        c1.subscribe(topic.clone()).timed(),
        c2.subscribe(topic.clone()).timed(),
    )
        .join()
        .await;

    let (sub1_time, _) = sub1_res?;
    let (sub2_time, _) = sub2_res?;

    let pub1_fut = c1
        .publish(
            topic.clone(),
            msg1,
            None,
            1100,
            Duration::from_secs(60),
            false,
        )
        .timed();
    let recv1_fut = c2.receive(&topic, msg1).timed();
    let (pub1_res, recv1_res) = (pub1_fut, recv1_fut).join().await;

    let (pub1_time, _) = pub1_res?;
    let (recv1_time, _) = recv1_res?;

    let pub2_fut = c2
        .publish(
            topic.clone(),
            msg2,
            None,
            1100,
            Duration::from_secs(60),
            false,
        )
        .timed();
    let recv2_fut = c1.receive(&topic, msg2).timed();
    let (pub2_res, recv2_res) = (pub2_fut, recv2_fut).join().await;

    let (pub2_time, _) = pub2_res?;
    let (recv2_time, _) = recv2_res?;

    tokio::time::sleep(Duration::from_millis(150)).await;

    Ok(TwoWayPing {
        publish1: pub1_time.total(),
        publish2: pub2_time.total(),
        subscribe1: sub1_time.total(),
        subscribe2: sub2_time.total(),
        receive1: recv1_time.total(),
        receive2: recv2_time.total(),
    })
}

#[derive(Debug)]
struct Pairing {
    propose: Duration,
    receive_propose_req: Duration,
    approve: Duration,
    receive_propose_resp: Duration,
    subscribe_session: Duration,
    receive_settle_req: Duration,
    publish_settle_resp: Duration,
    recieve_settle_resp: Duration,
    total: Duration,
}

async fn pairing(mut app: Client, mut wallet: Client) -> anyhow::Result<Pairing> {
    let total_time = Instant::now();

    let pairing_topic = Topic::generate();
    let session_topic = Topic::generate();

    let propose_req_msg = generate_message();
    let propose_req_msg = propose_req_msg.as_str();

    let (propose_time, _) = app
        .propose_session(pairing_topic.clone(), propose_req_msg, None)
        .timed()
        .await?;

    let (recv_propose_req_time, mut messages) = wallet
        .fetch_stream([pairing_topic.clone()])
        .try_collect::<Vec<_>>()
        .timed()
        .await?;

    let Some(msg) = messages.pop() else {
        anyhow::bail!("no messages available on pairing topic");
    };

    if msg.topic != pairing_topic || msg.message.as_ref() != propose_req_msg {
        anyhow::bail!("invalid session propose message");
    }

    let propose_resp_msg = generate_message();
    let propose_resp_msg = propose_resp_msg.as_str();
    let settle_req_msg = generate_message();
    let settle_req_msg = settle_req_msg.as_str();

    let (approve_time, _) = wallet
        .approve_session(
            pairing_topic.clone(),
            session_topic.clone(),
            propose_resp_msg,
            settle_req_msg,
        )
        .timed()
        .await?;

    let (recv_propose_resp_time, _) = app
        .receive(&pairing_topic, propose_resp_msg)
        .timed()
        .await?;

    let (sub_session_time, _) = app.subscribe(session_topic.clone()).timed().await?;
    let (recv_settle_req_time, _) = app.receive(&session_topic, settle_req_msg).timed().await?;

    let settle_resp_msg = generate_message();
    let settle_resp_msg = settle_resp_msg.as_str();

    let pub_settle_resp_fut = app
        .publish(
            session_topic.clone(),
            settle_resp_msg,
            None,
            1100,
            Duration::from_secs(60),
            false,
        )
        .timed();

    let recv_settle_resp_fut = wallet.receive(&session_topic, settle_resp_msg).timed();

    let (pub_settle_resp_res, recv_settle_resp_res) =
        (pub_settle_resp_fut, recv_settle_resp_fut).join().await;

    let (pub_settle_resp_time, _) = pub_settle_resp_res?;
    let (recv_settle_resp_time, _) = recv_settle_resp_res?;

    Ok(Pairing {
        propose: propose_time.total(),
        receive_propose_req: recv_propose_req_time.total(),
        approve: approve_time.total(),
        receive_propose_resp: recv_propose_resp_time.total(),
        subscribe_session: sub_session_time.total(),
        receive_settle_req: recv_settle_req_time.total(),
        publish_settle_resp: pub_settle_resp_time.total(),
        recieve_settle_resp: recv_settle_resp_time.total(),
        total: total_time.elapsed(),
    })
}

const REGIONS: [&'static str; 4] = ["eu-central-1", "us-east-1", "ap-southeast-1", "sa-east-1"];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _log_guard = log::init();
    let args = Args::from_args();

    for _ in 0..5 {
        let result = async {
            let c1 = Client::new("c1", &args.address, &args.project_id).await?;
            let c2 = Client::new("c2", &args.address, &args.project_id).await?;

            pairing(c1, c2).await
        }
        .await;

        match result {
            Ok(timing) => {
                tracing::info!(?timing, "pairing successful");
            }

            Err(err) => {
                tracing::warn!(?err, "pairing failed");
            }
        }
    }

    Ok(())
}
