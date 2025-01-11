use wither_data::packet::clientbound::STATUS_PONG_RESPONSE;
use wither_macros::wither_packet;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[wither_packet(STATUS_PONG_RESPONSE)]
pub struct PongResponse {
    pub payload: i64,
}

impl PongResponse {
    pub fn new(payload: i64) -> Self {
        Self { payload }
    }
}
