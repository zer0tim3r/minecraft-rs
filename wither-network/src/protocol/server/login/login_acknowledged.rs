use serde::{Deserialize, Serialize};
use wither_data::packet::serverbound::LOGIN_LOGIN_ACKNOWLEDGED;
use wither_macros::wither_packet;

// Acknowledgement to the Login Success packet sent to the server.
#[derive(Serialize, Deserialize)]
#[wither_packet(LOGIN_LOGIN_ACKNOWLEDGED)]
pub struct LoginAcknowledged {}

impl LoginAcknowledged {
    pub fn new() -> Self {
        Self {}
    }
}
