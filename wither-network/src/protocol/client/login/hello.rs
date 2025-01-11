use serde::{Deserialize, Serialize};
use wither_data::packet::clientbound::LOGIN_HELLO;
use wither_macros::wither_packet;

use crate::types::Binary;


#[derive(Serialize, Deserialize, Debug)]
#[wither_packet(LOGIN_HELLO)]
pub struct Hello {
    pub server_id: String, // 20
    pub public_key: Binary,
    pub challenge: Binary,
    pub should_authenticate: bool,
}

impl Hello {
    pub fn new(
        server_id: String,
        public_key: Binary,
        challenge: Binary,
        should_authenticate: bool,
    ) -> Self {
        Self {
            server_id,
            public_key,
            challenge,
            should_authenticate,
        }
    }
}