use serde::{Deserialize, Serialize};
use wither_data::packet::serverbound::STATUS_STATUS_REQUEST;
use wither_macros::wither_packet;

#[derive(Serialize, Deserialize)]
#[wither_packet(STATUS_STATUS_REQUEST)]
pub struct StatusRequest {
    // empty
}

impl StatusRequest {
    pub fn new() -> Self {
        Self {}
    }
}
