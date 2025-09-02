use std::num::ParseIntError;
use std::fmt::Display;
use std::io::Error as IOError;
use std::collections::BTreeMap;

use exdisj::{
    log_debug, log_error, log_info,
    io::log::LoggerBase
};
use tokio::fs::File;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use hmac::{Hmac, Mac as _};
use sha2::Sha256;
use rand_core::RngCore;
use jwt::{SignWithKey as _, VerifyWithKey as _};

use common::{
    loc::DAEMON_AUTH_KEY_PATH,
    jwt::{JwtContent, JwtRawContent},
    user::AuthKey
};

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

#[derive(Debug)]
pub struct SessionsManager<L: LoggerBase> {
    key: Hmac<Sha256>,
    buffer: AuthKey,
    logger: L
}
impl<L> SessionsManager<L> where L: LoggerBase {
    pub async fn open(logger: L) -> Result<Self, IOError> {
        let mut buffer: AuthKey = [0; 32];
        let mut file = match File::open(DAEMON_AUTH_KEY_PATH).await {
            Ok(f) => f,
            Err(e) => {
                log_error!(&logger, "Unable to open the authentication key.");
                return Err(e)
            }
        };
        if let Err(e) = file.read_exact(&mut buffer).await {
            log_error!(&logger, "Unable to read key contents, exactly 32 bytes.");
            return Err(e)
        }

        Ok(
            Self {
                key: Hmac::new_from_slice(&buffer).expect("the keysize is invalid..."),
                buffer,
                logger
            }
        )
    }
    pub fn new<R>(rng: &mut R, logger: L) -> Self where R: RngCore {
        let mut buffer: AuthKey = [0; 32];
        rng.fill_bytes(&mut buffer);
        log_info!(&logger, "Generating a new JWT session key.");

        Self {
            key: Hmac::new_from_slice(&buffer).expect("key size is invalid..."),
            buffer,
            logger
        }
    }
    pub async fn open_or_default<R>(rng: &mut R, logger: L) -> Self where R: RngCore {
        Self::open(logger.clone()).await.ok().unwrap_or_else(|| {
            Self::new(rng, logger)
        })
    }

    pub async fn save(&self) -> Result<(), IOError> {
        log_debug!(&self.logger, "Opening a file to save the JWT path at {DAEMON_AUTH_KEY_PATH}");
        let mut file = match File::create(DAEMON_AUTH_KEY_PATH).await {
            Ok(v) => v,
            Err(e) => {
                log_error!(&self.logger, "Unable to create a file to save the JWT key.");
                return Err(e);
            }
        };

        if let Err(e) = file.write_all(&self.buffer).await {
            log_error!(&self.logger, "Unable to save the JWT key to the file.");
            Err(e)
        }
        else {
            Ok( () )
        }
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

    let mut users = UserManager::default();
    let mut rng = rand::thread_rng();

    let to_store: JwtContent = {
        let user = users.create_user(&mut rng, "thing".to_string());
        user.get_jwt_content().to_content()
    };

    let sess = SessionsManager::new(&mut rng, ());
    let jwt = sess.make_jwt(to_store.clone()).expect("Unable to create the JWT");

    let decoded = sess.decode_jwt(&jwt).expect("unable to decode the jwt");
    assert_eq!(&to_store, &decoded);

    assert!( users.verify_user(&decoded)) 
}