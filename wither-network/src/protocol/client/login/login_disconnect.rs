use serde::{Deserialize, Serialize};
use wither_data::packet::clientbound::LOGIN_LOGIN_DISCONNECT;
use wither_macros::wither_packet;

#[derive(Serialize, Deserialize)]
#[wither_packet(LOGIN_LOGIN_DISCONNECT)]
pub struct LoginDisconnect {
    pub reason: String,
}

impl LoginDisconnect {
    // input json!
    pub fn new(reason: String) -> Self {
        Self { reason }
    }
}
