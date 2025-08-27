use std::collections::{HashMap, HashSet};
use std::io::{Error as IOError, ErrorKind};

use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use rand_core::RngCore;

use crate::auth::jwt::JwtBase;
use crate::auth::user::CompleteUserInformationMut;

use super::user::{UserInformation, CompleteUserInformation};

use common::loc::DAEMON_AUTH_USERS_PATH;

pub struct UserManagerIter<'a>(std::collections::hash_map::Iter<'a, u64, UserInformation>);
impl<'a> Iterator for UserManagerIter<'a> {
    type Item = CompleteUserInformation<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let inner = self.0.next()?;

        Some(
            inner.1.complete(*inner.0)
        )
    }
}

pub struct UserManagerIterMut<'a>(std::collections::hash_map::IterMut<'a, u64, UserInformation>);
impl<'a> Iterator for UserManagerIterMut<'a> {
    type Item = CompleteUserInformationMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let inner = self.0.next()?;

        Some(
            inner.1.complete_mut(*inner.0)
        )
    }
}

pub struct UserManagerRevokedIter<'a> {
    store: &'a HashMap<u64, UserInformation>,
    inner: std::collections::hash_set::Iter<'a, u64>
}
impl<'a> Iterator for UserManagerRevokedIter<'a> {
    type Item = CompleteUserInformation<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_id = self.inner.next()?;
        let target_user = self.store.get(next_id)?;

        Some(
            CompleteUserInformation::new(*next_id, target_user.auth_key(), target_user.nickname(), target_user.history())
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

    pub async fn open() -> Result<Self, IOError> {
        let mut file = File::open(DAEMON_AUTH_USERS_PATH).await?;
        Self::open_from(&mut file).await
    }
    pub async fn open_from<S>(stream: &mut S) -> Result<Self, IOError> where S: AsyncReadExt + Unpin {
        let mut contents = String::new();
        stream.read_to_string(&mut contents).await?;

        let mut as_json: Self = serde_json::from_str(&contents).map_err(|x| IOError::new(ErrorKind::InvalidData, x))?;

        as_json.curr_id = *as_json.users.keys().max().unwrap_or(&0);

        Ok( as_json )
    }
    pub async fn open_or_default() -> Self {
        Self::open().await.ok().unwrap_or_else(UserManager::default)
    }

    pub async fn save(&self) -> Result<(), std::io::Error> {
        let mut file = File::create(DAEMON_AUTH_USERS_PATH).await?;
        self.save_to(&mut file).await
    }
    pub async fn save_to<S>(&self, stream: &mut S) -> Result<(), IOError> where S: AsyncWriteExt + Unpin {
        let as_json = serde_json::to_string(&self).map_err(|x| IOError::new(ErrorKind::InvalidData, x))?;

        stream.write_all(as_json.as_bytes()).await?;

        Ok( () )
    }

    pub fn create_user<R>(&mut self, rng: &mut R, nickname: String) -> CompleteUserInformationMut<'_> 
        where R: RngCore {
        let new_id = self.curr_id + 1;
        self.curr_id += 1;

        let mut key: [u8; 32] = [0; 32];
        rng.fill_bytes(&mut key);
        let new_user = UserInformation::new(key, nickname, vec![]);

        self.users.insert(new_id, new_user);

        let target = self.users.get_mut(&new_id).expect("inserted user with id {new_id}, but was not able to get it back out");
        target.complete_mut(new_id)
    }
    pub fn revoke(&mut self, user: u64) {
        self.revoked.insert(user);
    } 
    pub fn is_revoked(&self, user: u64) -> bool {
        self.revoked.contains(&user)
    }

    pub fn get_user(&self, id: u64) -> Option<CompleteUserInformation<'_>> {
        let target = self.users.get(&id)?;
        Some( target.complete(id) )
    }
    pub fn get_user_mut(&mut self, id: u64) -> Option<CompleteUserInformationMut<'_>> {
        let target = self.users.get_mut(&id)?;
        Some( target.complete_mut(id) )
    }
    
    pub fn verify_user<T>(&self, jwt: &T) -> bool where T: JwtBase {
        match self.users.get(&jwt.id()) {
            Some(info) => {
                if self.is_revoked(jwt.id()) {
                    return false
                }

                info.auth_key() == jwt.key()
            }
            None => false
        }
    }
    pub fn verify_and_fetch_user<T>(&mut self, jwt: &T) -> Option<CompleteUserInformationMut<'_>> where T: JwtBase {
        if self.is_revoked(jwt.id()) {
            return None
        }

        let info = self.users.get_mut(&jwt.id())?;

        if info.auth_key() != jwt.key() { 
            None
        }
        else {
            Some( info.complete_mut(jwt.id()) )
        }
    }

    pub fn iter<'a>(&'a self) -> UserManagerIter<'a> {
        UserManagerIter(self.users.iter())
    }
    pub fn iter_mut<'a>(&'a mut self) -> UserManagerIterMut<'a> {
        UserManagerIterMut(self.users.iter_mut())
    }
    pub fn revoked_iter<'a>(&'a self) -> UserManagerRevokedIter<'a> {
        UserManagerRevokedIter {
            inner: self.revoked.iter(),
            store: &self.users
        }
    }
}

#[tokio::test]
async fn test_user_man() {
    use tokio::io::AsyncSeekExt;
    use super::jwt::JwtContent;

    let mut user_man = UserManager::new();
    let mut rng = rand::thread_rng();

    let key_to_test: JwtContent = {
        let user_one = user_man.create_user(&mut rng, "user-one".to_string());
        assert_eq!(user_one.nickname(), "user-one");
        user_one.get_jwt_content().to_content()
    };

    assert!( user_man.verify_user(&key_to_test) ); //Should always work

    user_man.revoke(key_to_test.id());

    assert!( !user_man.verify_user(&key_to_test) ); //Should not pass because it has been revoked

    //now we test for saving and whatnot
    let _ = tokio::fs::remove_file("user-man-test.json").await;
    let mut stream = File::create_new("user-man-test.json").await.unwrap();
    assert!( user_man.save_to(&mut stream).await.is_ok() );

    stream.rewind().await.expect("could not rewind");

    let new_user_man = UserManager::open_from(&mut stream).await.expect("could not re-open");

    assert_eq!(new_user_man, user_man);
}