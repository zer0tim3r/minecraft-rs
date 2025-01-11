use std::{
    collections::VecDeque,
    error::Error,
    sync::{atomic::AtomicBool, Arc},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

use wither_declare::*;
use wither_protocol::{
    self as protocol, packet_decoder::PacketDecoder, packet_encoder::PacketEncoder, Packet,
    PacketId, RawPacket,
};

pub struct Entity {
    pub id: i32,
    pub kind: entity::Kind,
}

pub struct Client {
    id: uuid::Uuid,
    reader: Arc<Mutex<tokio::net::tcp::OwnedReadHalf>>,
    writer: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
    encoder: Arc<Mutex<PacketEncoder>>,
    /// The packet decoder for incoming packets.
    decoder: Arc<Mutex<PacketDecoder>>,
    packet_notify: tokio::sync::Notify,
    pub packet_queue: Arc<Mutex<VecDeque<RawPacket>>>,
    pub living_entity: Option<Entity>,
    pub closed: AtomicBool,
}

impl Client {
    pub async fn new(host: &str, port: u16) -> Result<Self, Box<dyn Error>> {
        let host = match tokio::net::lookup_host(format!("{host}:{port}"))
            .await?
            .next()
        {
            Some(h) => h,
            _ => {
                return Err(Box::new(ClientError::InvalidHost));
            }
        };

        let connection = tokio::net::TcpStream::connect(host).await?;

        let (connection_reader, connection_writer) = connection.into_split();

        Ok(Self {
            id: uuid::Uuid::new_v4(),
            reader: Arc::new(Mutex::new(connection_reader)),
            writer: Arc::new(Mutex::new(connection_writer)),
            encoder: Arc::new(Mutex::new(PacketEncoder::default())),
            decoder: Arc::new(Mutex::new(PacketDecoder::default())),
            packet_notify: tokio::sync::Notify::new(),
            packet_queue: Arc::new(Mutex::new(VecDeque::new())),
            living_entity: None,
            closed: AtomicBool::new(false),
        })
    }

    pub async fn send_packet<P: Packet>(&self, packet: &P) -> Result<(), Box<dyn Error>> {
        let mut encoder = self.encoder.lock().await;
        encoder.append_packet(packet)?;

        let mut writer = self.writer.lock().await;
        let buf = encoder.take();
        log::debug!("Sending client packet id {}, {:02x}", P::PACKET_ID, buf);
        let _ = writer.write_all(&buf).await;

        /*
        writer
            .flush()
            .await
            .map_err(|_| PacketError::ConnectionWrite)?;
        */
        Ok(())
    }

    pub async fn peek_packet(&self) -> RawPacket {
        loop {
            if let Some(packet) = self.packet_queue.lock().await.pop_front() {
                return packet;
            }

            self.packet_notify.notified().await;
        }
    }

    pub fn close(&self) {
        self.closed
            .store(true, std::sync::atomic::Ordering::Relaxed);
        log::info!("Closed connection for {}", self.id);
    }

    pub async fn poll(&self) -> bool {
        loop {
            if self.closed.load(std::sync::atomic::Ordering::Relaxed) {
                // If we manually close (like a kick) we dont want to keep reading bytes
                return false;
            }

            let mut decoder = self.decoder.lock().await;

            match decoder.decode() {
                Ok(Some(packet)) => {
                    self.packet_queue.lock().await.push_back(packet);
                    self.packet_notify.notify_one();
                    // log::info!("{:?},", self.queue_packet.lock().await.send(packet.clone()));
                    // self.process_packet(&mut packet).await;
                    return true;
                }
                Ok(None) => (), //log::debug!("Waiting for more data to complete packet..."),
                Err(err) => {
                    log::warn!("Failed to decode packet for: {}", err.to_string());
                    self.close();
                    return false; // return to avoid reserving additional bytes
                }
            }

            decoder.reserve(4096);
            let mut buf = decoder.take_capacity();

            let bytes_read = self.reader.lock().await.read_buf(&mut buf).await;
            match bytes_read {
                Ok(cnt) => {
                    // log::debug!("Read {} bytes", cnt);
                    if cnt == 0 {
                        self.close();
                        return false;
                    }
                }
                Err(error) => {
                    log::error!("Error while reading incoming packet : {}", error);
                    self.close();
                    return false;
                }
            };

            // This should always be an O(1) unsplit because we reserved space earlier and
            // the call to `read_buf` shouldn't have grown the allocation.
            decoder.queue_bytes(buf);
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("cannot lookup host")]
    InvalidHost,
    #[error("client disconnected")]
    Disconnect,
    #[error("unknown client error")]
    Unknown,
}

pub struct ClientInstance {
    pub client: Arc<Client>,
}

impl ClientInstance {
    pub async fn new(host: &str, port: u16, username: &str) -> Result<Self, Box<dyn Error>> {
        let client = Arc::new(Client::new(host, port).await?);

        tokio::spawn({
            let client = client.clone();
            async move {
                loop {
                    if !client.poll().await {
                        break;
                    }
                }
            }
        });

        client
        .send_packet(&protocol::server::handshake::HandShake::new(
            768,
            "localhost".into(),
            25565,
            wither_protocol::ClientIntent::Login,
        ))
        .await
        .unwrap();

        /*
         * Handshake
         */
        client
            .send_packet(&protocol::server::login::Hello::new(
                username.into(),
                protocol::bytebuf::Uuid(uuid::Uuid::new_v4()),
            ))
            .await
            .unwrap();

        loop {
            let packet = match client.peek_packet().await {
                mut packet => {
                    match packet.id.0 {
                        protocol::client::login::Hello::PACKET_ID => {
                            println!("buf : {:02x}{:02x}", packet.id.0, packet.bytebuf);
                            protocol::client::login::Hello::read(&mut packet.bytebuf)?
                        }
                        _ => {
                            break;
                        }
                    }
                }
            };

            println!("Hello : {}, {:02x}, {:02x}", packet.server_id, bytes::Bytes::from(packet.public_key.0), bytes::Bytes::from(packet.challenge.0));

            let packet = client.peek_packet().await;
            println!("packet : {}", packet.id.0);

            break;
        }

        println!("break");


        Ok(Self { client })
    }
}
