use std::{fmt::Display, net::IpAddr};
use std::sync::{Arc, RwLock, RwLockReadGuard};

//use common::msg::PendingUser;
use common::user::CompleteUserInformation;
use exdisj::{
    log_error, log_info,
    io::log::{ChanneledLogger, LoggerBase},
    auth::{
        encrypt::RsaHandler,
        stream::RsaStream
    }
};
use once_cell::sync::OnceCell;
use rand::{rngs::StdRng, CryptoRng, SeedableRng};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, MutexGuard};
use common::{
    jwt::JwtContent,
    user::{UserHistoryElement, CompleteUserInformationMut}
};

//use super::app::{ApprovalsManager, ApprovalRequestFuture};

use super::{
    sess::{JwtDecodeError, SessionsManager},
    user_man::UserManager
};

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct ClientUserInformation {
    id: u64,
    jwt: String
}
impl ClientUserInformation {
    pub fn new(id: u64, jwt: String) -> Self {
        Self {
            id,
            jwt
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn jwt(&self) -> &str {
        &self.jwt
    }
}

#[derive(Debug)]
pub enum RenewalError {
    RevokedUser,
    NoSuchUser,
    JWT(jwt::Error)
}
impl Display for RenewalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: &dyn Display = match self {
            Self::RevokedUser => &"the user is revoked",
            Self::NoSuchUser => &"there is no user with that matching id",
            Self::JWT(j) => j
        };

        x.fmt(f)
    }
}
impl std::error::Error for RenewalError { }

#[derive(Debug)]
struct AuthManagerState<L> where L: LoggerBase {
    sess: SessionsManager<L>,
    user: UserManager<L>,
    //app: ApprovalsManager
}
impl<L> AuthManagerState<L> where L: LoggerBase {
    async fn open_or_default<R>(rng: &mut R, logger: L) -> Self where R: RngCore {
        log_info!(&logger, "Opening the authentication inner state");
        Self {
            sess: SessionsManager::open_or_default(rng, logger.clone()).await,
            user: UserManager::open_or_default(logger).await,
            //app: ApprovalsManager::default()
        }
    }
    async fn save(&self) -> Result<(), std::io::Error> {
        self.sess.save().await?;
        self.user.save().await
    }

    fn create_user<R>(&mut self, rng: &mut R, nickname: String) -> Result<ClientUserInformation, jwt::Error>
        where R: RngCore + CryptoRng {
            let user = self.user.create_user(rng, nickname);
            
            Ok(
                ClientUserInformation::new(
                    user.id(),
                    self.sess.make_jwt(user.get_jwt_content())?
                )
            )
    }
    fn renew_user(&self, id: u64) -> Result<String, RenewalError> {
        if self.user.is_revoked(id) {
            return Err( RenewalError::RevokedUser )
        }
        
        let user = match self.user.get_user(id) {
            Some(v) => v,
            None => return Err( RenewalError::NoSuchUser )
        };

        self.sess.make_jwt(user.get_jwt_content())
            .map_err(RenewalError::JWT)
    }
    fn is_user_revoked(&self, id: u64) -> bool {
        self.user.is_revoked(id)
    }

    fn decode_jwt(&self, jwt: &str) -> Result<JwtContent, JwtDecodeError> {
        self.sess.decode_jwt(jwt)
    }

    fn resolve_user_mut(&mut self, jwt: &str) -> Option<CompleteUserInformationMut<'_>> {
        let jwt = self.decode_jwt(jwt).ok()?;

        self.user.verify_and_fetch_user_mut(&jwt)
    }
    fn revoke_user(&mut self, id: u64) {
        self.user.revoke(id);
    }

    /*
    fn register_user_request(&mut self, from_ip: IpAddr) -> ApprovalRequestFuture {
        let request = self.app.register_request(from_ip);

        let future = ApprovalRequestFuture::new(**request);
        request.with_async(&future);

        future
    }
    fn approve_user<R>(&mut self, with_id: u64, nickname: String, rng: &mut R)  where R: RngCore + CryptoRng {
        
    }

    
    fn register_new_user(&mut self, from_ip: IpAddr) {
        self.app.register_new_user(from_ip)
    }
    fn pending_approvals(&self) -> Vec<&PendingUser> {
        self.app.pending()
    }
    fn approve_user<R>(&mut self, with_id: u64, nickname: String, rng: &mut R) -> Result<ClientUserInformation, ApprovalError>
        where R: RngCore + CryptoRng {
            let pending = self.app.approve_user(with_id)
                .ok_or(ApprovalError::UserNotFound(with_id))?;
            // We have to award it a new name & ID
            
            let new_user = 
    }
    fn deny_user(&mut self, with_id: u64) -> bool {
        self.app.deny_user(with_id)
    } 
    */
}

type AuthManState = Arc<RwLock<Option<AuthManagerState<ChanneledLogger>>>>;

pub struct AuthProvision<'a, L> where L: LoggerBase {
    inner: RwLockReadGuard<'a, Option<AuthManagerState<L>>>
}
impl<'a, L> AsRef<AuthManagerState<L>> for AuthProvision<'a, L> where L: LoggerBase {
    fn as_ref(&self) -> &AuthManagerState<L> {
        self.inner.as_ref().unwrap()
    }
}
impl<'a, L> AuthProvision<'a, L> where L: LoggerBase {
    fn new(inner: RwLockReadGuard<'a, Option<AuthManagerState<L>>>) -> Self {
        Self {
            inner
        }
    }

    pub fn get_all_users(&'a self) -> Vec<CompleteUserInformation<'a>> {
        let inner = self.as_ref();
        inner.user.iter().collect()
    }

    pub fn get_user_info(&'a self, id: u64) -> Option<CompleteUserInformation<'a>> {
        let inner = self.as_ref();
        inner.user.get_user(id)
    }
}

#[derive(Debug)]
pub struct AuthManager {
    rsa: Arc<RsaHandler>,
    rng: Arc<Mutex<StdRng>>,
    state: AuthManState,
    logger: ChanneledLogger
}
impl AuthManager {
    pub async fn new(logger: ChanneledLogger) -> Self {
        log_info!(&logger, "Opening the authentication manager");

        let mut rng = StdRng::from_rng(&mut rand::thread_rng()).expect("Unable to create a new stdrng");
        Self {
            rsa: Arc::new(RsaHandler::new(&mut rng).expect("unable to create an RSA key")),
            rng: Arc::new(Mutex::new(rng)),
            state: Arc::new(RwLock::new(None)),
            logger
        }
    }

    pub fn get_rsa(&self) -> &RsaHandler {
        &self.rsa
    }
    pub fn make_rsa_stream<S>(&self, stream: S) -> RsaStream<S, &RsaHandler> {
        RsaStream::new(stream, &self.rsa)
    }
    pub async fn get_rng(&self) -> MutexGuard<'_, StdRng> {
        self.rng.lock().await 
    }

    pub async fn initialize(&self) {
        let rng = &mut *self.get_rng().await;
        let core = AuthManagerState::open_or_default(rng, self.logger.clone()).await;
        let mut guard = match self.state.write() {
            Ok(g) => g,
            Err(e) => e.into_inner()
        };

        *guard = Some(core)
    }

    #[allow(clippy::await_holding_lock)]
    pub async fn save(&self) -> Result<(), std::io::Error> {
        let guard = self.state.read()
            .expect("the inner state for authentication was corrupted");

        let state = (*guard).as_ref().expect("the authentication system is not initialized");
        log_info!(&self.logger, "Saving the auth session.");
        if let Err(e) = state.save().await {
            log_error!(&self.logger, "Unable to save the authentication manager due to '{:?}'", &e);
            Err(e)
        }
        else {
            log_info!(&self.logger, "Save complete");
            Ok(())
        }
    }

    pub fn get_provision(&self) -> AuthProvision<'_, ChanneledLogger> {
        let guard = self.state.read()
            .expect("the inner state for authentication was corrupted");
        AuthProvision::new(guard)
    }

    #[allow(clippy::await_holding_lock)]
    pub async fn create_user(&self, nickname: String) -> Result<ClientUserInformation, jwt::Error> {
        let mut guard = self.state.write()
            .expect("the inner state for authentication was corrupted");

        let state = (*guard).as_mut().expect("the authentication system is not initialized");
        state.create_user(&mut *self.get_rng().await, nickname)
    }
    pub fn renew_user(&self, id: u64) -> Result<String, RenewalError> {
        let guard = self.state.read()
            .expect("the inner state for authentication was corrupted");

        let state = (*guard).as_ref().expect("the authentication system is not initialized");
        state.renew_user(id)
    }

    pub fn is_user_revoked(&self, id: u64) -> bool {
        let guard = self.state.read()
            .expect("the inner state for authentication was corrupted");

        let state = (*guard).as_ref().expect("the authentication system is not initialized");
        state.is_user_revoked(id)
    }
    pub fn revoke_user(&self, id: u64) {
        let mut guard = self.state.write()
            .expect("the inner state for authentication was corrupted");

        let state = (*guard).as_mut().expect("the authentication system is not initialized");

        state.revoke_user(id);
    }
    pub fn sign_user_in(&self, jwt: &str, from_ip: IpAddr) -> bool {
        let mut guard = self.state.write()
            .expect("the inner state for authentication was corrupted");

        let state = (*guard).as_mut().expect("the authentication system is not initialized");

        let mut user = match state.resolve_user_mut(jwt) {
            Some(u) => u,
            None => return false
        };

        let history = UserHistoryElement::new(from_ip, chrono::Utc::now());
        user.add_to_history(history);
        true
    }
}

pub static AUTH: OnceCell<AuthManager> = OnceCell::new();

#[tokio::test]
async fn test_auth_manager() {
    use exdisj::io::log::{Logger, Prefix, ConsoleColor, LoggerLevel, LoggerRedirect};

    let _ = tokio::fs::remove_file("auth_man.log").await;
    let logger = Logger::new("auth_man.log", LoggerLevel::Debug, LoggerRedirect::default()).unwrap();
    let channel  = logger.make_channel(Prefix::new_const("Auth", ConsoleColor::Red));
    let inner = AuthManager::new(channel).await;

    let auth = AUTH.get_or_init(|| inner);
    auth.initialize().await;

    auth.save().await.expect("Unable to save the keys");
}