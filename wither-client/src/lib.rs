use std::{
    collections::VecDeque,
    error::Error,
    sync::{atomic::AtomicBool, Arc},
};

use rand::Rng;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{Mutex, OnceCell, RwLock},
};

use rsa::{BigUint, Pkcs1v15Encrypt, RsaPublicKey};

use dashmap::DashMap;

use wither_declare::*;
use wither_network::{
    packet_decoder::PacketDecoder, packet_encoder::PacketEncoder, protocol, ClientIntent,
    CompressionLevel, CompressionThreshold, ConnectionProtocol, Packet, PacketId, RawPacket,
};

pub struct Entity {
    pub id: i32,
    pub kind: entity::Kind,
}

pub struct RawClient {
    id: Arc<std::sync::Mutex<Option<uuid::Uuid>>>,
    reader: Arc<Mutex<tokio::net::tcp::OwnedReadHalf>>,
    writer: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
    encoder: Arc<Mutex<PacketEncoder>>,
    /// The packet decoder for incoming packets.
    decoder: Arc<Mutex<PacketDecoder>>,

    intent: Arc<OnceCell<ClientIntent>>,
    pub protocol: Arc<RwLock<ConnectionProtocol>>,

    notify: Arc<DashMap<String, tokio::sync::Notify>>,

    packet_queue: Arc<Mutex<VecDeque<RawPacket>>>,

    pub living_entity: Option<Entity>,

    pub closed: AtomicBool,
}

impl RawClient {
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

        let notify = DashMap::<String, tokio::sync::Notify>::new();

        notify.insert("close".into(), tokio::sync::Notify::new());
        notify.insert("packet".into(), tokio::sync::Notify::new());
        notify.insert("login".into(), tokio::sync::Notify::new());
        // notify.insert("on_connect".into(), tokio::sync::Notify::new());

        // notify.insert("on_connect".into(), tokio::sync::Notify::new());
        // notify.get("on_connect".into()).unwrap().notify_one();

        Ok(Self {
            id: Arc::new(std::sync::Mutex::new(None)),
            reader: Arc::new(Mutex::new(connection_reader)),
            writer: Arc::new(Mutex::new(connection_writer)),
            encoder: Arc::new(Mutex::new(PacketEncoder::default())),
            decoder: Arc::new(Mutex::new(PacketDecoder::default())),
            intent: Arc::new(OnceCell::new()),
            protocol: Arc::new(RwLock::new(ConnectionProtocol::HandShake)),
            packet_queue: Arc::new(Mutex::new(VecDeque::new())),

            notify: Arc::new(notify),
            living_entity: None,
            closed: AtomicBool::new(false),
        })
    }

    pub async fn set_protocol(&self, intent: ClientIntent) -> Result<(), Box<dyn Error>> {
        if self.intent.initialized() {
            return Err("client intent initialization must be once".into());
        }

        self.intent
            .get_or_init(|| async {
                self.send_packet(&protocol::server::handshake::HandShake::new(
                    768,
                    "localhost".into(),
                    25565,
                    intent,
                ))
                .await
                .expect("cannot send serverbound::Handshake packet");

                *self.protocol.write().await = match intent {
                    ClientIntent::Status => ConnectionProtocol::Status,
                    ClientIntent::Login => ConnectionProtocol::Login,
                    ClientIntent::Transfer => {
                        unimplemented!()
                    }
                };

                intent
            })
            .await;

        Ok(())
    }

    pub async fn attempt_login(&self, name: &str) -> Result<(), Box<dyn Error>> {
        match self.intent.get() {
            Some(intent) => match intent {
                ClientIntent::Login => {
                    self.send_packet(&protocol::server::login::Hello::new(
                        name.into(),
                        wither_network::types::Uuid(uuid::Uuid::new_v4()),
                    ))
                    .await?;

                    let login = self.get_notify("login");
                    let close = self.get_notify("close");

                    tokio::select! {
                        _ = login.notified() => Ok(()),
                        _ = close.notified() => Err("client has been disconnected".into()),
                    }
                }
                _ => Err("client intent must be ClientIntent::Login".into()),
            },
            None => Err("client intent is not initialized".into()),
        }
    }

    pub async fn send_packet<P: Packet>(&self, packet: &P) -> Result<(), Box<dyn Error>> {
        let mut encoder = self.encoder.lock().await;
        encoder.append_packet(packet)?;

        let mut writer = self.writer.lock().await;
        let buf = encoder.take();
        let _ = writer.write_all(&buf).await;

        log::debug!(
            "Wrote client packet {:?}, {} ({:02x}{:02x})",
            self.protocol.read().await,
            P::PACKET_ID,
            P::PACKET_ID,
            buf
        );

        /*
        writer
            .flush()
            .await
            .map_err(|_| PacketError::ConnectionWrite)?;
        */
        Ok(())
    }

    pub async fn peek_packet(&self) -> Option<RawPacket> {
        loop {
            self.get_notify("packet").notified().await;

            if let Some(packet) = self.packet_queue.lock().await.pop_front() {
                return Some(packet);
            } else {
                break None;
            }
        }
    }

    async fn process_packet(&self, packet: &mut RawPacket) -> Result<(), Box<dyn Error>> {
        self.packet_queue.lock().await.push_back(packet.clone());
        self.get_notify("packet").notify_one();

        let protocol = *self.protocol.read().await;

        match protocol {
            ConnectionProtocol::HandShake => match packet.id.0 {
                _ => {
                    unimplemented!()
                }
            },
            ConnectionProtocol::Status => match packet.id.0 {
                protocol::client::status::StatusResponse::PACKET_ID => {
                    todo!()
                }
                protocol::client::status::PongResponse::PACKET_ID => {
                    todo!()
                }
                _ => {
                    unimplemented!()
                }
            },
            ConnectionProtocol::Login => match packet.id.0 {
                protocol::client::login::Hello::PACKET_ID => {
                    let packet = protocol::client::login::Hello::read(&mut packet.bytebuf)?;

                    let (n, e) = rsa_der::public_key_from_der(&packet.public_key.inner)?;

                    let public_key =
                        RsaPublicKey::new(BigUint::from_bytes_be(&n), BigUint::from_bytes_be(&e))?;

                    println!("public_key : {:?}", public_key);

                    let mut rng = rand::rngs::OsRng;

                    let symmetric_key = rng.gen::<[u8; 16]>();

                    self.decoder
                        .lock()
                        .await
                        .set_encryption(Some(&symmetric_key));

                    let encrypted_key =
                        public_key.encrypt(&mut rng, Pkcs1v15Encrypt, &symmetric_key)?;

                    let encrypted_challenge =
                        public_key.encrypt(&mut rng, Pkcs1v15Encrypt, &packet.challenge.inner)?;

                    self.send_packet(&protocol::server::login::Key::new(
                        encrypted_key,
                        encrypted_challenge,
                    ))
                    .await?;

                    self.encoder
                        .lock()
                        .await
                        .set_encryption(Some(&symmetric_key));
                }
                protocol::client::login::LoginCompression::PACKET_ID => {
                    let packet =
                        protocol::client::login::LoginCompression::read(&mut packet.bytebuf)?;

                    self.decoder.lock().await.set_compression(true);
                    self.encoder.lock().await.set_compression(Some((
                        CompressionThreshold(packet.compression_threshold.0 as u32),
                        CompressionLevel(6),
                    )))?;

                    // self.send_packet(protocol::client::)
                }
                protocol::client::login::LoginFinished::PACKET_ID => {
                    let packet = protocol::client::login::LoginFinished::read(&mut packet.bytebuf)?;

                    *self.id.lock().unwrap() = Some(packet.id.0);

                    self.send_packet(&protocol::server::login::LoginAcknowledged::new())
                        .await?;

                    *self.protocol.write().await = ConnectionProtocol::Config;

                    self.get_notify("login").notify_waiters();

                    // self.send_packet(protocol::client::)
                }
                protocol::client::login::LoginDisconnect::PACKET_ID => {
                    let packet = protocol::client::login::LoginDisconnect::read(&mut packet.bytebuf)?;

                    log::warn!("received disconnect packet! reason: {}", packet.reason);
                }
                _ => {
                    unimplemented!()
                }
            },
            _ => {}
        }

        Ok(())

        // println!("clientbound::Hello : {:?}", packet);

        // let packet = match client.peek_packet().await {
        //     Some(mut packet) => packet,
        //     _ => {
        //         break;
        //     }
        // };
    }

    pub fn get_notify(
        &self,
        name: &str,
    ) -> dashmap::mapref::one::Ref<'_, String, tokio::sync::Notify> {
        self.notify.get(name).unwrap()
    }

    pub fn close(&self) {
        self.closed
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.get_notify("close").notify_waiters();

        log::info!(
            "Closed connection for {}",
            self.id.lock().unwrap().unwrap_or_default()
        );
    }

    pub async fn poll(&self) -> bool {
        loop {
            if self.closed.load(std::sync::atomic::Ordering::Relaxed) {
                // If we manually close (like a kick) we dont want to keep reading bytes
                return false;
            }

            let protocol = *self.protocol.read().await;
            let decoded = self.decoder.lock().await.decode();

            match decoded {
                Ok(Some(packet)) => match self.process_packet(&mut packet.clone()).await {
                    Ok(()) => {
                        log::info!(
                            "Success to process packet: {:?}, {} ({:02x}{:02x})",
                            protocol,
                            packet.id.0,
                            packet.id.0,
                            packet.bytebuf,
                        );
                        return true;
                    }
                    Err(err) => {
                        log::warn!("Failed to process packet for: {:?}", err);
                    }
                },
                Ok(None) => (), //log::debug!("Waiting for more data to complete packet..."),
                Err(err) => {
                    log::warn!("Failed to decode packet for: {}", err.to_string());
                    self.close();
                    return false; // return to avoid reserving additional bytes
                }
            }

            let mut decoder = self.decoder.lock().await;
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

pub struct Client {
    pub raw_client: Arc<RawClient>,
}

impl Client {
    pub async fn new(host: &str, port: u16, username: &str) -> Result<Self, Box<dyn Error>> {
        let raw_client = Arc::new(RawClient::new(host, port).await?);

        tokio::spawn({
            let client = raw_client.clone();
            async move {
                loop {
                    tokio::select! {
                        polling = client.poll() => if !polling {
                            break;
                        },


                    }
                }
            }
        });

        raw_client.set_protocol(ClientIntent::Login).await?;
        raw_client.attempt_login(username).await?;

        Ok(Self { raw_client })
    }
}
