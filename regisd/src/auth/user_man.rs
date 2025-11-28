use std::collections::{HashMap, HashSet};
use std::io::{Error as IOError, ErrorKind};

use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize, de::{self, Visitor, SeqAccess, MapAccess}};
use rand_core::RngCore;
use exdisj::{
    log_info, log_error, 
    io::log::LoggerBase
};
use common::{
    jwt::JwtBase,
    user::{CompleteUserInformationMut, UserInformation, CompleteUserInformation}
};

use common::loc::DAEMON_AUTH_USERS_PATH;

pub struct UserManagerIter<'a>(std::collections::hash_map::Iter<'a, u64, UserInformation>);
impl<'a> Iterator for UserManagerIter<'a> {
    type Item = CompleteUserInformation<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let inner = self.0.next()?;

        Some(
            #[allow(deprecated)]
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
            #[allow(deprecated)]
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

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum UserManagerFields {
    Users,
    Revoked
}

struct UserManagerVisitor;
impl<'de> Visitor<'de> for UserManagerVisitor {
    type Value = UserManager<()>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct UserManager<()>")
    }
    
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>, {
        let users: HashMap<u64, UserInformation> = seq.next_element()?
                .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let revoked = seq.next_element()?
                .ok_or_else(|| de::Error::invalid_length(1, &self))?;

        let max_id = *users.keys().max().unwrap_or(&0);
        Ok(
            UserManager {
                users,
                revoked,
                curr_id: max_id,
                logger: ()
            }
        )
    }
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>, {
        let mut users: Option<HashMap<u64, UserInformation>> = None;
        let mut revoked = None;

        while let Some(key) = map.next_key()? {
            match key {
                UserManagerFields::Users => {
                    if users.is_some() {
                        return Err(de::Error::duplicate_field("users"));
                    }

                    users = Some( map.next_value()? )
                }
                UserManagerFields::Revoked => {
                    if revoked.is_some() {
                        return Err(de::Error::duplicate_field("revoked"));
                    }

                    revoked = Some( map.next_value()? )
                }
            }
        }

        let users = users
                .ok_or_else(|| de::Error::missing_field("users"))?;
        let revoked = revoked
                .ok_or_else(|| de::Error::missing_field("revoked"))?;

        let max_id = *users.keys().max().unwrap_or(&0);
        Ok(
            UserManager {
                users,
                revoked,
                curr_id: max_id,
                logger: ()
            }
        )
    }
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct UserManager<L> where L: LoggerBase {
    users: HashMap<u64, UserInformation>,
    revoked: HashSet<u64>,
    #[serde(skip)]
    curr_id: u64,
    #[serde(skip)]
    logger: L
}
impl<'de> Deserialize<'de> for UserManager<()> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de> {
        const FIELDS: &[&str] = &["users", "revoked"];

        deserializer.deserialize_struct("UserManager", FIELDS, UserManagerVisitor)
    }
}
impl Default for UserManager<()> {
    fn default() -> Self {
         Self {
            users: HashMap::new(),
            revoked: HashSet::new(),
            curr_id: 0,
            logger: ()
        }
    }
}
impl<L> UserManager<L> where L: LoggerBase {
    pub fn change_logger<L2>(self, new: L2) -> UserManager<L2> where L2: LoggerBase {
        UserManager {
            users: self.users,
            revoked: self.revoked,
            curr_id: self.curr_id,
            logger: new
        }
    }
    
    pub async fn open(logger: L) -> Result<Self, IOError> {
        let mut file = match File::open(DAEMON_AUTH_USERS_PATH).await {
            Ok(v) => v,
            Err(e) => {
                log_error!(&logger, "Unable to open the user manager file '{:?}'", &e);
                return Err(e)
            }
        };

        Self::open_from(&mut file, logger).await
    }
    pub async fn open_from<S>(stream: &mut S, logger: L) -> Result<Self, IOError> where S: AsyncReadExt + Unpin {
        let mut bytes: Vec<u8> = vec![];
        if let Err(e) = stream.read_to_end(&mut bytes).await {
            log_error!(&logger, "Unable to read file contents from the stream '{:?}'", &e);
            return Err(e);
        }

        let as_json: UserManager<()> = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(e) => {
                log_error!(&logger, "Unable to decode the binary data into a UserManager instance: '{:?}'", &e);
                return Err( IOError::new(ErrorKind::InvalidData, e) );
            }
        };

        Ok( as_json.change_logger(logger) )
    }
    pub async fn open_or_default(logger: L) -> Self {
        match Self::open(logger.clone()).await.ok() {
            Some(v) => v,
            None => {
                log_info!(&logger, "Opening as a default user manager.");
                UserManager::default().change_logger(logger)
            }
        }
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
        log_info!(&self.logger, "Creating a new user with id '{new_id}' and nickname '{}'", &nickname);

        let mut key: [u8; 32] = [0; 32];
        rng.fill_bytes(&mut key);
        let new_user = UserInformation::new(key, nickname, vec![]);

        self.users.insert(new_id, new_user);

        let target = self.users.get_mut(&new_id).expect("inserted user with id {new_id}, but was not able to get it back out");
        #[allow(deprecated)]
        target.complete_mut(new_id)
    }
    pub fn delete_user(&mut self, id: u64) -> Option<UserInformation> {
        self.users.remove(&id)
    }
    pub fn revoke(&mut self, user: u64) {
        log_info!(&self.logger, "Revoking user with id '{user}'");
        self.revoked.insert(user);
    } 
    pub fn is_revoked(&self, user: u64) -> bool {
        self.revoked.contains(&user)
    }

    pub fn get_user(&self, id: u64) -> Option<CompleteUserInformation<'_>> {
        let target = self.users.get(&id)?;
        #[allow(deprecated)]
        Some( target.complete(id) )
    }
    pub fn get_user_mut(&mut self, id: u64) -> Option<CompleteUserInformationMut<'_>> {
        let target = self.users.get_mut(&id)?;
        #[allow(deprecated)]
        Some( target.complete_mut(id) )
    }
    
    pub fn verify_user<T>(&self, jwt: &T) -> bool where T: JwtBase {
        match self.users.get(&jwt.id()) {
            Some(info) => {
                if self.is_revoked(jwt.id()) {
                    log_info!(&self.logger, "A user with id '{}' does exist, but it is revoked.", jwt.id());
                    return false;
                }

                info.auth_key() == jwt.key()
            }
            None => false
        }
    }
    pub fn verify_and_fetch_user<T>(&self, jwt: &T) -> Option<CompleteUserInformation<'_>> 
        where T: JwtBase {
            if self.is_revoked(jwt.id()) {
                return None
            }

            let info = self.users.get(&jwt.id())?;

            if info.auth_key() != jwt.key() { 
                None
            }
            else {
                #[allow(deprecated)]
                Some( info.complete(jwt.id()) )
            }
    }
    pub fn verify_and_fetch_user_mut<T>(&mut self, jwt: &T) -> Option<CompleteUserInformationMut<'_>> 
        where T: JwtBase {
            if self.is_revoked(jwt.id()) {
                return None
            }

            let info = self.users.get_mut(&jwt.id())?;

            if info.auth_key() != jwt.key() { 
                None
            }
            else {
                #[allow(deprecated)]
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
    use common::jwt::JwtContent;

    let mut user_man = UserManager::default();
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

    let new_user_man = UserManager::open_from(&mut stream, ()).await.expect("could not re-open");

    assert_eq!(new_user_man, user_man);
}