use std::sync::Arc;

use tokio::io::AsyncReadExt;
use wither_protocol as protocol;

#[tokio::main(worker_threads = 128)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));

    log::info!("Hello World!");

    let instance = wither_client::ClientInstance::new("localhost", 25565, "bot").await?;


    // client.send_packet(&protocol::server::status::StatusRequest::new()).await.unwrap();
    // client.send_packet(&protocol::server::status::PingRequest::new(0)).await.unwrap();
    // client.send_packet(&protocol::server::status::PingRequest::new(512)).await.unwrap();


    Ok(())
}