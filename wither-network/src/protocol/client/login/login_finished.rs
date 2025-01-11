use serde::{Deserialize, Serialize};
use wither_data::packet::clientbound::LOGIN_LOGIN_FINISHED;
use wither_macros::wither_packet;

use crate::types::{PropertyMap, Uuid};

#[derive(Serialize, Deserialize, Debug)]
#[wither_packet(LOGIN_LOGIN_FINISHED)]
pub struct LoginFinished {
    pub id: Uuid,
    pub name: String, // 16
    pub properties: PropertyMap,
}

impl LoginFinished {
    pub fn new(id: Uuid, name: String, properties: PropertyMap) -> Self {
        Self {
            id,
            name,
            properties,
        }
    }
}
