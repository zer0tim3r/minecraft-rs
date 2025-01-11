use serde::{Deserialize, Serialize};
use wither_data::packet::serverbound::LOGIN_KEY;
use wither_macros::wither_packet;

use crate::types::Binary;

#[derive(Serialize, Deserialize)]
#[wither_packet(LOGIN_KEY)]
pub struct Key {
    pub keybytes: Binary,
    pub encrypted_challenge: Binary,
}

impl Key {
    pub fn new(keybytes: Vec<u8>, encrypted_challenge: Vec<u8>) -> Self {
        Self {
            keybytes: Binary::new(keybytes),
            encrypted_challenge: Binary::new(encrypted_challenge)
        }
    }
}