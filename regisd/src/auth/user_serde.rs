use serde::{ser::SerializeStruct, Deserialize, Serialize};
use serde::de::{self, SeqAccess, MapAccess, Visitor};

use base64::prelude::{Engine as _, BASE64_STANDARD};

use super::user::{AuthKey, UserInformation};

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum UserInformationFields {
    AuthKey,
    Nickname,
    History
}
struct UserInformationVisitor;
impl<'de> Visitor<'de> for UserInformationVisitor {
    type Value = UserInformation;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct UserInformation")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>, {
        let auth_key_raw: &'de str = seq.next_element()?
                .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let nickname= seq.next_element()?
                .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        let history= seq.next_element()?
                .ok_or_else(|| de::Error::invalid_length(2, &self))?;

        let mut auth_key: AuthKey = [0; 32];
        BASE64_STANDARD.decode_slice(auth_key_raw, &mut auth_key)
                .map_err(de::Error::custom)?;
        
        Ok(
            UserInformation::new(auth_key, nickname, history)
        )
    }
    
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>, {
        
        let mut auth_key_raw: Option<&'de str> = None;
        let mut nickname = None;
        let mut history = None;

        while let Some(key) = map.next_key()? {
            match key {
                UserInformationFields::AuthKey => {
                    if auth_key_raw.is_some() {
                        return Err( de::Error::duplicate_field("authkey"));
                    }

                    auth_key_raw = Some( map.next_value()? );
                },
                UserInformationFields::Nickname => {
                    if nickname.is_some() {
                        return Err( de::Error::duplicate_field("nickname"));
                    }

                    nickname = Some( map.next_value()? );
                },
                UserInformationFields::History => {
                    if history.is_some() {
                        return Err( de::Error::duplicate_field("history"));
                    }

                    history = Some( map.next_value()? );
                },
            }
        }

        let auth_key_raw = auth_key_raw
            .ok_or_else(|| de::Error::missing_field("authkey"))?;
        let nickname = nickname
            .ok_or_else(|| de::Error::missing_field("nickname"))?;
        let history = history
            .ok_or_else(|| de::Error::missing_field("history"))?;

        let mut auth_key: AuthKey = [0; 32];
        BASE64_STANDARD.decode_slice(auth_key_raw, &mut auth_key)
                .map_err(de::Error::custom)?;
        
        Ok(
            UserInformation::new(auth_key, nickname, history)
        )
    }
}


impl Serialize for UserInformation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        
        let mut ser = serializer.serialize_struct("UserInformation", 3)?;
        ser.serialize_field("authkey", &BASE64_STANDARD.encode(self.auth_key()))?;
        ser.serialize_field("nickname", self.nickname())?;
        ser.serialize_field("history", self.history())?;

        ser.end()
    }
}
impl<'de> Deserialize<'de> for UserInformation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        const FIELDS: &[&str] = &["authkey", "nickname", "history"];

        deserializer.deserialize_struct("UserInformation", FIELDS, UserInformationVisitor)
    }
}