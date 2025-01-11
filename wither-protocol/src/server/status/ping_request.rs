use serde::{Deserialize, Serialize};
use wither_data::packet::serverbound::STATUS_PING_REQUEST;
use wither_macros::wither_packet;

#[derive(Serialize, Deserialize)]
#[wither_packet(STATUS_PING_REQUEST)]
pub struct PingRequest {
    pub payload: i64,
}

impl PingRequest {
    pub fn new(payload: i64) -> Self {
        Self {
            payload
        }
    }
}