use std::num::ParseIntError;
use std::fmt::Display;
use std::io::Error as IOError;
use std::collections::BTreeMap;

use tokio::fs::File;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use hmac::{Hmac, Mac as _};
use sha2::Sha256;
use rand_core::RngCore;
use jwt::{SignWithKey as _, VerifyWithKey as _};

use common::loc::DAEMON_AUTH_KEY_PATH;

use crate::auth::jwt::{JwtContent, JwtRawContent};

use super::user::AuthKey;

#[derive(Debug)]
pub enum JwtDecodeError {
    MissingField(&'static str),
    JWT(jwt::Error),
    NumParse(ParseIntError),
    Decode(base64::DecodeSliceError)
}
impl Display for JwtDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: &dyn Display = match self {
            Self::MissingField(n) => return write!(f, "the field '{n}' is missing"),
            Self::JWT(j) => j,
            Self::NumParse(n) => n,
            Self::Decode(v) => v
        };

        x.fmt(f)
    }
}
impl std::error::Error for JwtDecodeError { }
impl From<jwt::Error> for JwtDecodeError {
    fn from(value: jwt::Error) -> Self {
        Self::JWT(value)
    }
}
impl From<ParseIntError> for JwtDecodeError {
    fn from(value: ParseIntError) -> Self {
        Self::NumParse(value)
    }
}
impl From<&'static str> for JwtDecodeError {
    fn from(value: &'static str) -> Self {
        Self::MissingField(value)
    }
}
impl From<base64::DecodeSliceError> for JwtDecodeError {
    fn from(value: base64::DecodeSliceError) -> Self {
        Self::Decode(value)
    }
}

pub struct SessionsManager {
    key: Hmac<Sha256>
}
impl SessionsManager {
    pub async fn try_open() -> Result<Self, IOError> {
        let mut buffer: AuthKey = [0; 32];
        let mut file = File::open(DAEMON_AUTH_KEY_PATH).await?;
        file.read_exact(&mut buffer).await?;

        Ok(
            Self {
                key: Hmac::new_from_slice(&buffer).expect("the keysize is invalid...")
            }
        )
    }
    pub async fn generate_key<R>(rng: &mut R) -> Result<Self, IOError> where R: RngCore {
        let mut buffer: AuthKey = [0; 32];
        rng.fill_bytes(&mut buffer);

        let mut file = File::create(DAEMON_AUTH_KEY_PATH).await?;
        file.write_all(&buffer).await?;

        Ok(
            Self {
                key: Hmac::new_from_slice(&buffer).expect("key size is invalid...")
            }
        )
    }

    pub fn make_jwt<V>(&self, content: V) -> Result<String, jwt::Error> where V: Into<JwtRawContent> {
        let mut coll = BTreeMap::new();
        let content: JwtRawContent = content.into();
        coll.insert("id", content.id().to_string());
        coll.insert("key", content.take_key());

        coll.sign_with_key(&self.key)
    }
    pub fn decode_jwt(&self, jwt: &str) -> Result<JwtContent, JwtDecodeError> {
        let coll: BTreeMap<String, String> = jwt.verify_with_key(&self.key).map_err(JwtDecodeError::from)?;

        let id = coll.get("id")
            .ok_or(JwtDecodeError::from("id"))?
            .parse()
            .map_err(JwtDecodeError::from)?;
        let key = coll.get("key")
            .ok_or(JwtDecodeError::from("key"))?
            .clone();

        let as_raw = JwtRawContent::new(id, key);
        as_raw.try_into().map_err(JwtDecodeError::from)
    } 
}

#[tokio::test]
async fn test_sess_man() {
    use super::user_man::UserManager;

    let mut users = UserManager::new();
    let mut rng = rand::thread_rng();

    let to_store: JwtContent = {
        let user = users.create_user(&mut rng, "thing".to_string());
        user.get_jwt_content().to_content()
    };

    let sess = SessionsManager::generate_key(&mut rng).await.expect("unable to create a session manager");
    let jwt = sess.make_jwt(to_store.clone()).expect("Unable to create the JWT");

    let decoded = sess.decode_jwt(&jwt).expect("unable to decode the jwt");
    assert_eq!(&to_store, &decoded);

    assert!( users.verify_user(&decoded)) 
}