use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

pub type AuthKey = [u8; 32];

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserHistoryElement {
    from_ip: IpAddr,
    at_time: DateTime<Utc>
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UserInformation {
    auth_key: AuthKey,
    nickname: String,
    history: Vec<UserHistoryElement>
}
impl UserInformation {
    pub fn new(auth_key: AuthKey, nickname: String, history: Vec<UserHistoryElement>) -> Self {
        Self {
            auth_key,
            nickname,
            history
        }
    }

    pub fn auth_key(&self) -> &AuthKey {
        &self.auth_key
    }
    pub fn nickname(&self) -> &str {
        &self.nickname
    }
    pub fn nickname_mut(&mut self) -> &mut String {
        &mut self.nickname
    }
    pub fn history(&self) -> &[UserHistoryElement] {
        &self.history
    }
    pub fn history_mut(&mut self) -> &mut Vec<UserHistoryElement> {
        &mut self.history
    }

    pub fn set_nickname(&mut self, new: String) {
        self.nickname = new
    }
    pub fn add_to_history(&mut self, new: UserHistoryElement) {
        self.history.push(new);
    }

    pub(super) fn complete<'a>(&'a self, id: u64) -> CompleteUserInformation<'a> {
        CompleteUserInformation {
            id,
            auth_key: &self.auth_key,
            nickname: &self.nickname,
            history: &self.history
        }
    }
    pub(super) fn complete_mut<'a>(&'a mut self, id: u64) -> CompleteUserInformationMut<'a> {
        CompleteUserInformationMut {
            id,
            auth_key: &self.auth_key,
            nickname: &mut self.nickname,
            history: &mut self.history
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct CompleteUserInformation<'a> {
    id: u64,
    auth_key: &'a AuthKey,
    nickname: &'a str,
    history: &'a [UserHistoryElement]
}
impl<'a> CompleteUserInformation<'a> {
    pub fn new(id: u64, auth_key: &'a AuthKey, nickname: &'a str, history: &'a [UserHistoryElement]) -> Self {
        Self {
            id,
            auth_key,
            nickname,
            history
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn auth_key(&self) -> &AuthKey {
        &self.auth_key
    }
    pub fn nickname(&self) -> &str {
        &self.nickname
    }
    pub fn history(&self) -> &[UserHistoryElement] {
        &self.history
    }
}
impl PartialEq<UserInformation> for CompleteUserInformation<'_> {
    fn eq(&self, other: &UserInformation) -> bool {
        self.auth_key  == other.auth_key() && self.history == other.history() && self.nickname == other.nickname()
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct CompleteUserInformationMut<'a> {
    id: u64,
    auth_key: &'a AuthKey,
    nickname: &'a mut String,
    history: &'a mut Vec<UserHistoryElement>
}
impl<'a> CompleteUserInformationMut<'a> {
    pub fn new(id: u64, auth_key: &'a AuthKey, nickname: &'a mut String, history: &'a mut Vec<UserHistoryElement>) -> Self {
        Self {
            id,
            auth_key,
            nickname,
            history
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn auth_key(&self) -> &AuthKey {
        &self.auth_key
    }
    pub fn nickname(&self) -> &str {
        &self.nickname
    }
    pub fn history(&self) -> &[UserHistoryElement] {
        &self.history
    }

    pub fn set_nickname(&mut self, new: String) {
        *self.nickname = new
    }
    pub fn add_to_history(&mut self, new: UserHistoryElement) {
        self.history.push(new);
    }
}