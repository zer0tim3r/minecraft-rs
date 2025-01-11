#[tokio::main(worker_threads = 4)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));

    log::info!("Hello World!");

    let instance = wither_client::Client::new("localhost", 25565, "bot").await?;

    // client.send_packet(&protocol::server::status::StatusRequest::new()).await.unwrap();
    // client.send_packet(&protocol::server::status::PingRequest::new(0)).await.unwrap();
    // client.send_packet(&protocol::server::status::PingRequest::new(512)).await.unwrap();

    instance.raw_client.get_notify("close").notified().await;

    Ok(())
}
