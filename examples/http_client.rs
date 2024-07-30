use {
    relay_client::{http::Client, ConnectionOptions},
    relay_rpc::{
        auth::{ed25519_dalek::SigningKey, AuthToken},
        domain::Topic,
    },
    std::{sync::Arc, time::Duration},
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::from_args();

    let key1 = SigningKey::generate(&mut rand::thread_rng());
    let client1 = Client::new(&create_conn_opts(&key1, &args.address, &args.project_id))?;

    let key2 = SigningKey::generate(&mut rand::thread_rng());
    let client2 = Client::new(&create_conn_opts(&key2, &args.address, &args.project_id))?;

    let topic = Topic::generate();
    let message: Arc<str> = Arc::from("Hello WalletConnect!");

    client1
        .publish(
            topic.clone(),
            message.clone(),
            None,
            1100,
            Duration::from_secs(30),
            false,
        )
        .await?;

    println!("[client1] published message with topic: {topic}",);

    tokio::time::sleep(Duration::from_secs(1)).await;

    let messages = client2.fetch(topic).await?.messages;
    let message = messages
        .first()
        .ok_or(anyhow::anyhow!("fetch did not return any messages"))?;

    println!("[client2] received message: {}", message.message);

    Ok(())
}
