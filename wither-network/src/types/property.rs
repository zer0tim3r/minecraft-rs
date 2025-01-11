use serde::{
    de::{SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::codec::var_int::VarInt;

// basically game profile
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Property {
    pub name: String,
    // base 64
    pub value: String,
    // base 64
    pub signature: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PropertyMap {
    inner: Vec<Property>,
}

impl PropertyMap {
    pub fn new(inner: Vec<Property>) -> Self {
        Self { inner }
    }
}

impl Serialize for PropertyMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.inner.len() + 1))?;

        seq.serialize_element(&VarInt(self.inner.len() as i32))?;

        for b in &self.inner {
            seq.serialize_element(&b)?;
        }

        seq.end()
    }
}

impl<'de> Deserialize<'de> for PropertyMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PropertyMapVisitor;

        impl<'de> Visitor<'de> for PropertyMapVisitor {
            type Value = PropertyMap;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid Binary encoded in a byte sequence")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                match seq.next_element::<VarInt>()? {
                    Some(varint) => {
                        let mut out: Vec<Property> = vec![];

                        for _ in 0..varint.0 {
                            if let Some(byte) = seq.next_element::<Property>()? {
                                out.push(byte);
                            } else {
                                return Err(serde::de::Error::custom(
                                    "Binary was ended incompletely",
                                ));
                            }
                        }

                        Ok(PropertyMap::new(out))
                    }
                    _ => Err(serde::de::Error::custom("invalid varint"))
                }
            }
        }

        deserializer.deserialize_seq(PropertyMapVisitor)
    }
}
