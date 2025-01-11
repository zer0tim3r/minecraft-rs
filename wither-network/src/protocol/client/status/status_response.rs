use wither_data::packet::clientbound::STATUS_STATUS_RESPONSE;
use wither_macros::wither_packet;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[wither_packet(STATUS_STATUS_RESPONSE)]
pub struct StatusResponse {
    pub json_response: String, // 32767
}

impl StatusResponse {
    pub fn new(json_response: String) -> Self {
        Self { json_response }
    }
}
