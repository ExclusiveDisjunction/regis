use serde::{Serialize, ser::SerializeStruct, Deserialize, de::{self, Visitor}};
use base64::{Engine, prelude::BASE64_STANDARD};
use aes_gcm::Nonce;

use crate::auth::encrypt::EncryptedAesMessage;

use super::err::InvalidNonceLengthError;

pub struct AesPacket<B> {
    cipher: B,
    nonce: B
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum AesPacketFields {
    Cipher,
    Nonce
}
struct AesPacketVisitor;
impl<'de> Visitor<'de> for AesPacketVisitor {
    type Value = AesPacket<&'de str>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("AesPacket<&'de str>")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>, {
        let cipher: &'de str = seq.next_element()?
                .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let nonce: &'de str = seq.next_element()?
                .ok_or_else(|| de::Error::invalid_length(1, &self))?;

        Ok(
            AesPacket::new(
                cipher,
                nonce
            )
        )
    }
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>, {
        
        let mut cipher = None;
        let mut nonce = None;

        while let Some(key) = map.next_key()? {
            match key {
                AesPacketFields::Cipher => {
                    if cipher.is_some() {
                        return Err( de::Error::duplicate_field("cipher") );
                    }

                    cipher = Some( map.next_value()? )
                },
                AesPacketFields::Nonce => {
                    if nonce.is_some() {
                        return Err( de::Error::duplicate_field("nonce") );
                    }

                    nonce = Some( map.next_value()? )
                }
            }
        }

        let cipher = cipher
            .ok_or_else(|| de::Error::missing_field("cipher") )?;
        let nonce =  nonce
            .ok_or_else(|| de::Error::missing_field("nonce") )?;

        Ok(
            AesPacket::new(
                cipher,
                nonce
            )
        )
    }
}


impl<B> Serialize for AesPacket<B> where B: Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut ser = serializer.serialize_struct("AesPacket", 2)?;
        ser.serialize_field("cipher", &self.cipher)?;
        ser.serialize_field("nonce", &self.nonce)?;

        ser.end()
    }
}
impl<'de> Deserialize<'de> for AesPacket<&'de str> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        let fields = &["cipher", "nonce"];

        deserializer.deserialize_struct("AesPacket", fields, AesPacketVisitor)
    }
}
impl<'de> Deserialize<'de> for AesPacket<String> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        let imm: AesPacket<&'de str> = AesPacket::deserialize(deserializer)?;

        Ok(
            Self {
                cipher: imm.cipher.to_string(),
                nonce: imm.nonce.to_string()
            }
        )
    }
}
impl<B> AesPacket<B> {
    pub fn new(cipher: B, nonce: B) -> Self {
        Self {
            cipher,
            nonce
        }
    }
    
    pub fn cipher(&self) -> &B {
        &self.cipher
    }
    pub fn nonce(&self) -> &B {
        &self.nonce
    }
}
impl<B> AesPacket<B> where B: AsRef<[u8]> {
    pub fn decode(self) -> Result<AesPacket<Vec<u8>>, base64::DecodeError> {
            Ok( 
                AesPacket::new(
                    BASE64_STANDARD.decode(self.cipher)?,
                    BASE64_STANDARD.decode(self.nonce)?
                )
            )
    }
}
impl AesPacket<Vec<u8>> {
    pub fn unwrap(self) -> Result<EncryptedAesMessage, InvalidNonceLengthError> {
        if self.nonce.len() != 12 { //The exact size of a nonce
            Err(InvalidNonceLengthError)
        }
        else {
            Ok( EncryptedAesMessage::new(self.cipher, Nonce::from_exact_iter(self.nonce).unwrap() ) )
        }
    }

    pub fn encode(self) -> AesPacket<String> {
        AesPacket {
            cipher: BASE64_STANDARD.encode(self.cipher),
            nonce: BASE64_STANDARD.encode(self.nonce)
        }
    }
}

impl From<EncryptedAesMessage> for AesPacket<Vec<u8>> {
    fn from(value: EncryptedAesMessage) -> Self {
        let nonce = value.nonce().to_vec();
        Self {
            cipher: value.take_cipher(),
            nonce
        }
    }
}