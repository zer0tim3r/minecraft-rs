use bytes::{Buf, BufMut};

use wither_data::packet::serverbound::HANDSHAKE_INTENTION;
use wither_macros::wither_packet;

use crate::{
    bytebuf::{ByteBuf, ByteBufMut, ReadingError}, ClientIntent, Packet, VarInt
};

#[wither_packet(HANDSHAKE_INTENTION)]
pub struct HandShake {
    pub protocol_version: VarInt,
    pub host_name: String, // 255
    pub port: u16,
    pub intention: ClientIntent,
}

impl HandShake {
    pub fn new(protocol_version: i32, host_name: String, port: u16, intention: ClientIntent) -> Self {
        Self {
            protocol_version: VarInt(protocol_version),
            host_name,
            port,
            intention,
        }
    }
}

impl Packet for HandShake {
    fn read(bytebuf: &mut impl Buf) -> Result<Self, ReadingError> {
        Ok(Self {
            protocol_version: bytebuf.try_get_var_int()?,
            host_name: bytebuf.try_get_string_len(255)?,
            port: bytebuf.try_get_u16()?,
            intention: bytebuf
                .try_get_var_int()?
                .try_into()
                .map_err(|_| ReadingError::Message("Invalid Intent".to_string()))?,
        })
    }

    fn write(&self, bytebuf: &mut impl BufMut) {
        bytebuf.put_var_int(&self.protocol_version);
        bytebuf.put_string_len(&self.host_name, 255);
        bytebuf.put_u16(self.port);
        bytebuf.put_var_int(&VarInt(self.intention as i32));
    }
}
