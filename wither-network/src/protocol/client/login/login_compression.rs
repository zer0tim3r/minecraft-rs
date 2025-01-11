use wither_data::packet::clientbound::LOGIN_LOGIN_COMPRESSION;
use wither_macros::wither_packet;
use serde::{Deserialize, Serialize};

use crate::VarInt;

#[derive(Serialize, Deserialize, Debug)]
#[wither_packet(LOGIN_LOGIN_COMPRESSION)]
pub struct LoginCompression {
    pub compression_threshold: VarInt,
}

impl LoginCompression {
    pub fn new(compression_threshold: VarInt) -> Self {
        Self { compression_threshold }
    }
}
