// Added by LDemetrios

use base64::Engine;
use base64::engine::general_purpose;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug)]
pub(crate) struct Base64Bytes(pub Vec<u8>);

impl Serialize for Base64Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string = general_purpose::STANDARD.encode(self.0.as_slice());
        serializer.serialize_str(string.as_str())
    }
}

impl<'de> Deserialize<'de> for Base64Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        Ok(Base64Bytes(general_purpose::STANDARD.decode(string).unwrap()))
    }
}
