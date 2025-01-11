use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{SeqAccess, Visitor};
use uuid::Uuid as UuidInner;
use std::fmt;

#[derive(Debug)]
pub struct Uuid(pub UuidInner);

impl Serialize for Uuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        
        serializer.serialize_u128(self.0.as_u128())
    }
}


impl<'de> Deserialize<'de> for Uuid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct UuidVisitor;
        
        impl<'de> Visitor<'de> for UuidVisitor {
            type Value = Uuid;
        
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a 16-byte array representing a UUID")
            }
        
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut out: [u8; 16] = [0; 16];

                for i in 0..16 {
                    if let Some(byte) = seq.next_element::<u8>()? {
                        out[i] = byte;
                    } else {
                        return Err(serde::de::Error::custom("Uuid was ended incompletely"));
                    }
                }

                Ok(Uuid(UuidInner::from_bytes(out)))
            }
        }

        deserializer.deserialize_seq(UuidVisitor)
    }
}
