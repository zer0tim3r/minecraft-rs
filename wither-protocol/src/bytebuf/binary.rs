use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::codec::var_int::VarInt;
use crate::codec::Codec;

pub struct Binary(pub Vec<u8>);

impl Serialize for Binary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Binary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BinaryVisitor;

        impl<'de> Visitor<'de> for BinaryVisitor {
            type Value = Binary;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid Binary encoded in a byte sequence")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let val = {
                    let mut i = 0;
                    let mut val = 0;
                    loop {
                        if VarInt::MAX_SIZE.get() <= i {
                            return Err(serde::de::Error::custom("VarInt was too large"));
                        }

                        if let Some(byte) = seq.next_element::<u8>()? {
                            val |= (i32::from(byte) & 0b01111111) << (i * 7);
                            if byte & 0b10000000 == 0 {
                                break val;
                            }
                        } else {
                            return Err(serde::de::Error::custom("VarInt was too large"));
                        }

                        i += 1;
                    }
                };

                let mut out: Vec<u8> = vec![];

                for _ in 0..val {
                    if let Some(byte) = seq.next_element::<u8>()? {
                        out.push(byte);
                    } else {
                        return Err(serde::de::Error::custom("Binary was ended incompletely"));
                    }
                }

                Ok(Binary(out))
            }
        }

        deserializer.deserialize_seq(BinaryVisitor)
    }
}
