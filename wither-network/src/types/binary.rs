use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::codec::var_int::VarInt;

#[derive(Debug)]
pub struct Binary {
    pub inner: Vec<u8>,
}

impl Binary {
    pub fn new(v: Vec<u8>) -> Self {
        Self { inner: v }
    }
}

impl Serialize for Binary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.inner.len() + 1))?;

        seq.serialize_element(&VarInt(self.inner.len() as i32))?;

        seq.serialize_element(self.inner.as_slice())?;

        seq.end()
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
                match seq.next_element::<VarInt>()? {
                    Some(varint) => {
                        let mut out: Vec<u8> = vec![];

                        for _ in 0..varint.0 {
                            if let Some(byte) = seq.next_element::<u8>()? {
                                out.push(byte);
                            } else {
                                return Err(serde::de::Error::custom("Binary was ended incompletely"));
                            }
                        }
        
                        Ok(Binary::new(out))
                    }
                    _ => Err(serde::de::Error::custom("invalid varint"))
                }
                
            }
        }

        deserializer.deserialize_seq(BinaryVisitor)
    }
}