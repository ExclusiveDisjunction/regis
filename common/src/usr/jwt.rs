use std::fmt::Debug;

use base64::prelude::{Engine, BASE64_STANDARD};

use super::user::{CompleteUserInformation, CompleteUserInformationMut, AuthKey};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct JwtRawContent {
    id: u64,
    key: String
}
impl JwtRawContent {
    pub fn new(id: u64, key: String) -> Self {
        Self {
            id,
            key
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn take_key(self) -> String {
        self.key
    }
}

pub trait JwtBase: Debug + PartialEq + Eq + Clone {
    fn id(&self) -> u64;
    fn key(&self) -> &AuthKey;
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct JwtContent {
    id: u64,
    key: AuthKey
}
impl TryFrom<JwtRawContent> for JwtContent {
    type Error = base64::DecodeSliceError;
    fn try_from(value: JwtRawContent) -> Result<Self, Self::Error> {
        let mut key: AuthKey = [0; 32];
        BASE64_STANDARD.decode_slice(value.key, &mut key)?;

        Ok(
            Self {
                id: value.id,
                key
            }
        )
    }
}
impl From<JwtContent> for JwtRawContent {
    fn from(value: JwtContent) -> Self {
        let key = BASE64_STANDARD.encode(value.key);

        Self {
            id: value.id,
            key
        }
    }
}
impl JwtBase for JwtContent {
    fn id(&self) -> u64 {
        self.id
    }
    fn key(&self) -> &AuthKey {
        &self.key
    }
}
impl JwtContent {
    pub fn new(id: u64, key: AuthKey) -> Self {
        Self {
            id,
            key
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct JwtContentRef<'a> {
    id: u64,
    key: &'a AuthKey
}
impl From<JwtContentRef<'_>> for JwtRawContent {
    fn from(value: JwtContentRef<'_>) -> Self {
        let key = BASE64_STANDARD.encode(value.key);

        Self {
            id: value.id,
            key
        }
    }
}
impl JwtBase for JwtContentRef<'_> {
    fn id(&self) -> u64 {
        self.id
    }
    fn key(&self) -> &AuthKey {
        self.key
    }
}
impl<'a> JwtContentRef<'a> {
    pub fn new(id: u64, key: &'a AuthKey) -> Self {
        Self {
            id,
            key
        }
    }

    pub fn to_content(self) -> JwtContent {
        JwtContent { id: self.id, key: *self.key }
    }
}

impl<'a> CompleteUserInformation<'a> {
    pub fn get_jwt_content(&'a self) -> JwtContentRef<'a> {
        JwtContentRef { id: self.id(), key: self.auth_key() }
    }
}
impl<'a> CompleteUserInformationMut<'a> {
    pub fn get_jwt_content(&'a self) -> JwtContentRef<'a> {
        JwtContentRef { id: self.id(), key: self.auth_key() }
    }
}