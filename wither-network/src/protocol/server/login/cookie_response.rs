use crate::{
    bytebuf::{ByteBuf, ByteBufMut, ReadingError},
    codec::identifier::Identifier,
    Packet, VarInt,
};
use bytes::{Buf, BufMut};
use wither_data::packet::serverbound::LOGIN_COOKIE_RESPONSE;
use wither_macros::wither_packet;

#[wither_packet(LOGIN_COOKIE_RESPONSE)]
/// Response to a Cookie Request (login) from the server.
/// The Notchian server only accepts responses of up to 5 kiB in size.
pub struct CookieResponse {
    pub key: Identifier,
    pub payload: Option<bytes::Bytes>, // 5120,
}

const MAX_COOKIE_LENGTH: usize = 5120;

impl Packet for CookieResponse {
    fn read(bytebuf: &mut impl Buf) -> Result<Self, ReadingError> {
        let key = bytebuf.try_get_identifer()?;
        let has_payload = bytebuf.try_get_bool()?;

        if !has_payload {
            return Ok(Self {
                key,
                payload: None,
            });
        }

        let payload_length = bytebuf.try_get_var_int()?;
        let length = payload_length.0;

        let payload = bytebuf.try_copy_to_bytes_len(length as usize, MAX_COOKIE_LENGTH)?;

        Ok(Self {
            key,
            payload: Some(payload),
        })
    }

    fn write(&self, bytebuf: &mut impl BufMut) {
        bytebuf.put_identifier(&self.key);

        match &self.payload {
            Some(payload) => {
                bytebuf.put_bool(true);

                if payload.len() > MAX_COOKIE_LENGTH {
                    // Should be panic?, I mean its our fault
                    panic!("Cookie is too big");
                }

                bytebuf.put_var_int(&VarInt(payload.len() as i32));
                bytebuf.put_slice(payload);
            }
            _ => {
                bytebuf.put_bool(false);
            }
        }

    }
}
