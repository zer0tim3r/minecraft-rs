use serde::{Deserialize, Serialize};
use crate::bytebuf::Uuid;
use wither_data::packet::serverbound::LOGIN_HELLO;
use wither_macros::wither_packet;

#[derive(Serialize, Deserialize)]
#[wither_packet(LOGIN_HELLO)]
pub struct Hello {
    pub name: String, // 16
    pub profile_id: Uuid,
}

impl Hello {
    pub fn new(name: String, profile_id: Uuid) -> Self {
        Self {
            name,
            profile_id
        }
    }
}