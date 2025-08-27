use std::{collections::{HashMap, HashSet}, net::IpAddr};

use chrono::Utc;
use common::loc::DAEMON_AUTH_USERS_PATH;
use rsa_ext::{RsaPublicKey, RsaPrivateKey, PaddingScheme, PublicKey};
use aes_gcm::{AesGcm, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncReadExt as _, AsyncWriteExt as _}
};

pub type AuthKey = [u8; 32];

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserHistoryElement {
    from_ip: IpAddr,
    at_time: chrono::DateTime<Utc>
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    pub fn history(&self) -> &[UserHistoryElement] {
        &self.history
    }

    pub fn set_nickname(&mut self, new: String) {
        self.nickname = new
    }
    pub fn add_to_history(&mut self, new: UserHistoryElement) {
        self.history.push(new);
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize)]
pub struct CompleteUserInformation<'a> {
    id: u64,
    auth_key: &'a AuthKey,
    nickname: &'a str,
    history: &'a [UserHistoryElement]
}
impl CompleteUserInformation<'_> {
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

pub struct UserManagerIter<'a>(std::collections::hash_map::Iter<'a, u64, UserInformation>);
impl<'a> Iterator for UserManagerIter<'a> {
    type Item = CompleteUserInformation<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let inner = self.0.next()?;

        Some(
            CompleteUserInformation {
                id: *inner.0, 
                auth_key: &inner.1.auth_key,
                nickname: &inner.1.nickname,
                history: &inner.1.history
            }
        )
    }
}


#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct UserManager {
    users: HashMap<u64, UserInformation>,
    revoked: HashSet<u64>,
    #[serde(skip)]
    curr_id: u64
}
impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}
impl UserManager {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            revoked: HashSet::new(),
            curr_id: 0
        }
    }

    pub async fn open() -> Result<Self, std::io::Error> {
        let mut file = match File::open(DAEMON_AUTH_USERS_PATH).await {
            Ok(f) => f,
            Err(_) => {
               // We will just return a blank file...
               return Ok( Self::new() )
            }
        };

        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;

        let as_json: Self = serde_json::from_str(&contents).map_err(|x| std::io::Error::new(std::io::ErrorKind::InvalidData, x))?;

        Ok( as_json )
    }
    pub async fn save(&self) -> Result<(), std::io::Error> {
        let as_json = serde_json::to_string(&self).map_err(|x| std::io::Error::new(std::io::ErrorKind::InvalidData, x))?;

        let mut file = File::create(DAEMON_AUTH_USERS_PATH).await?;
        file.write_all(as_json.as_bytes()).await?;

        Ok( () )
    }

    pub fn create_user(&mut self, rng: &mut rand::rngs::ThreadRng, nickname: String) -> &mut UserInformation {
        let new_id = self.curr_id + 1;
        self.curr_id += 1;

        let mut key: [u8; 32] = [0; 32];
        rng.fill_bytes(&mut key);
        let new_user = UserInformation::new(key, nickname, vec![]);

        self.users.insert(new_id, new_user);

        self.users.get_mut(&new_id).expect("inserted user with id {new_id}, but was not able to get it back out")
    }
    pub fn revoke(&mut self, user: u64) {
        self.revoked.insert(user);
    } 
    pub fn is_revoked(&self, user: u64) -> bool {
        self.revoked.contains(&user)
    }
    
    pub fn verify_user(&self, id: u64, key: &AuthKey) -> bool {
        match self.users.get(&id) {
            Some(info) => {
                if self.is_revoked(id) {
                    return false
                }

                &info.auth_key == key
            }
            None => false
        }
    }

    pub fn iter<'a>(&'a self) -> UserManagerIter<'a> {
        UserManagerIter(self.users.iter())
    }
    pub fn revoked_iter<'a>(&'a self) -> std::collections::hash_set::Iter<'a, u64> {
        self.revoked.iter()
    }
}