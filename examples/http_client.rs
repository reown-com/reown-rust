use {
    relay_client::{
        http::{Client, WatchRegisterRequest, WatchUnregisterRequest},
        ConnectionOptions,
    },
    relay_rpc::{
        auth::{
            ed25519_dalek::{Keypair, PublicKey},
            rand,
            AuthToken,
        },
        domain::AuthSubject,
        rpc::{WatchStatus, WatchType},
    },
    std::time::Duration,
    structopt::StructOpt,
    url::Url,
};

#[derive(StructOpt)]
struct Args {
    /// Specify HTTP address.
    #[structopt(short, long, default_value = "https://relay.walletconnect.com/rpc")]
    address: String,

    /// Specify WalletConnect project ID.
    #[structopt(short, long, default_value = "3cbaa32f8fbf3cdcc87d27ca1fa68069")]
    project_id: String,
}

fn create_conn_opts(key: &Keypair, address: &str, project_id: &str) -> ConnectionOptions {
    let aud = Url::parse(address)
        .unwrap()
        .origin()
        .unicode_serialization();

    let auth = AuthToken::new(AuthSubject::generate())
        .aud(aud)
        .ttl(Duration::from_secs(60 * 60))
        .as_jwt(key)
        .unwrap();

    ConnectionOptions::new(project_id, auth).with_address(address)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::from_args();

    let key1 = Keypair::generate(&mut rand::thread_rng());
    let client1 = Client::new(&create_conn_opts(&key1, &args.address, &args.project_id))?;

    let relay_key: PublicKey = client1
        .watch_register(
            WatchRegisterRequest {
                service_url: "https://example.com".to_owned(),
                webhook_url: "https://example.com/webhook".to_owned(),
                watch_type: WatchType::Subscriber,
                tags: vec![1100],
                statuses: vec![WatchStatus::Delivered],
                ttl: Duration::from_secs(86400),
            },
            &key1,
        )
        .await?
        .into();

    println!("watch registered: relay_key={:?}", relay_key);

    client1
        .watch_unregister(
            WatchUnregisterRequest {
                service_url: "https://example.com".to_owned(),
                webhook_url: "https://example.com/webhook".to_owned(),
                watch_type: WatchType::Subscriber,
            },
            &key1,
        )
        .await?;

    println!("watch unregistered");

    Ok(())
}
