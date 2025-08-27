
use exdisj::io::log::ChanneledLogger;
use rand::{rngs::StdRng, SeedableRng};
use rand_core::RngCore;

use crate::auth::encrypt::RsaHandler;

use super::{
    sess::{JwtDecodeError, SessionsManager},
    jwt::{JwtBase, JwtContent},
    user::{AuthKey, UserInformation},
    user_man::UserManager,
    encrypt::RsaEncrypter
};

use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard};
use std::io::Error as IOError;

#[derive(Debug)]
struct AuthManagerState {
    sess: SessionsManager,
    user: UserManager
}
impl AuthManagerState {
    async fn open() -> Result<Self, IOError> {
        Ok(
            Self {
                sess: SessionsManager::open().await?,
                user: UserManager::open().await?
            }
        )
    }
    async fn open_or_default<R>(rng: &mut R) -> Self where R: RngCore {
        Self {
            sess: SessionsManager::open_or_default(rng).await,
            user: UserManager::open_or_default().await
        }
    }
}

#[derive(Debug)]
pub struct AuthManager {
    rsa: Arc<RsaHandler>,
    rng: Arc<Mutex<StdRng>>,
    state: Arc<RwLock<Option<AuthManagerState>>>
}
impl Default for AuthManager {
    fn default() -> Self {
        let mut rng = StdRng::from_rng(&mut rand::thread_rng()).expect("Unable to create a new stdrng");
        Self {
            rsa: Arc::new(RsaHandler::new(&mut rng).expect("unable to create an RSA key")),
            rng: Arc::new(Mutex::new(rng)),
            state: Arc::new(RwLock::new(None))
        }
    }
}
impl AuthManager {


    pub fn get_rsa(&self) -> &RsaHandler {
        &self.rsa
    }
    pub fn get_rng(&self) -> MutexGuard<'_, StdRng> {
        match self.rng.lock() {
            Ok(guard) => guard,
            Err(e) => {
                let mut guard = e.into_inner();
                *guard = StdRng::from_rng(&mut rand::thread_rng()).expect("Unable to create a new stdrng");

                guard
            }
        }
    }
}

lazy_static::lazy_static!{
    pub static ref AUTH: AuthManager = AuthManager::default();
}