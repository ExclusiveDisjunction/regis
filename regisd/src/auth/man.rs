use std::{fmt::Display, net::IpAddr};
use std::sync::Arc;

use chrono::Utc;
use common::jwt::JwtBase;
use common::msg::PendingUser;
//use common::msg::PendingUser;
use common::user::{CompleteUserInformation, UserHistoryElement};
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

//use super::app::{ApprovalsManager, ApprovalRequestFuture};

use crate::auth::app::{ApprovalRequestFuture, ApprovalsManager};
use crate::auth::sess::JwtDecodeError;

use super::{
    sess::SessionsManager,
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

pub(crate) struct AuthApprovalSession<'a, L: LoggerBase> {
    inner: &'a mut AuthManagerState<L>
}
impl<'a, L: LoggerBase> AuthApprovalSession<'a, L> {
    fn new(inner: &'a mut AuthManagerState<L>) -> Self {
        Self {
            inner
        }
    }

    #[inline]
    pub(crate) fn pending(&self) -> Vec<&PendingUser> {
        self.inner.app.pending()
    }

    pub(crate) fn register_request(&mut self, from_ip: IpAddr) -> ApprovalRequestFuture {
        let request = self.inner.app.register_request(from_ip);

        let future = ApprovalRequestFuture::new(**request);
        request.with_async(&future);

        future
    }
    pub(crate) fn approve_user<R>(&mut self, user_id: u64, nickname: String, rng: &mut R) -> Option<ClientUserInformation>
    where R: RngCore + CryptoRng {
        if !self.inner.app.contains_pending(user_id) {
            return None;
        }

        let new_user = self.inner.user.create_user(rng, nickname);
        if self.inner.app.approve_user(user_id, new_user.id()).is_none() {
            let id = new_user.id();
            drop(new_user);
            self.inner.user.delete_user(id);
            return None;
        };

        Some(
            ClientUserInformation::new(
                new_user.id(),
                self.inner.sess.make_jwt(new_user.get_jwt_content()).ok()?
            )
        )
    }
    pub(crate) fn deny(&mut self, user_id: u64) -> bool {
        self.inner.app.deny_user(user_id)
    }
}

#[derive(Debug)]
pub(crate) struct AuthManagerState<L> where L: LoggerBase {
    sess: SessionsManager<L>,
    user: UserManager<L>,
    app: ApprovalsManager
}
impl<L> AuthManagerState<L> where L: LoggerBase {
    async fn open_or_default<R>(rng: &mut R, logger: L) -> Self where R: RngCore {
        log_info!(&logger, "Opening the authentication inner state");
        Self {
            sess: SessionsManager::open_or_default(rng, logger.clone()).await,
            user: UserManager::open_or_default(logger).await,
            app: ApprovalsManager::default()
        }
    }
    async fn save(&self) -> Result<(), std::io::Error> {
        self.sess.save().await?;
        self.user.save().await
    }

    pub(crate) fn all_users(&self) -> Vec<CompleteUserInformation<'_>> {
        self.user.iter().collect()
    }
    #[inline]
    pub(crate) fn user_info(&self, id: u64) -> Option<CompleteUserInformation<'_>> {
        self.user.get_user(id)
    }

    fn create_new_user<R>(&mut self, rng: &mut R, nickname: String) -> Result<ClientUserInformation, jwt::Error> 
    where R: RngCore + CryptoRng {
        let user = self.user.create_user(rng, nickname);

        Ok(
            ClientUserInformation::new(
                user.id(),
                self.sess.make_jwt(user.get_jwt_content())?
            )
        )
    }
    /// If the user is not revoked, renew their JWT token, and return the content of it.
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

    /// Determines if a user, by ID, is revoked.
    #[inline]
    pub(crate) fn is_user_revoked(&self, id: u64) -> bool {
        self.user.is_revoked(id)
    }
    #[inline]
    pub(crate) fn approvals<'a>(&'a mut self) -> AuthApprovalSession<'a, L> {
        AuthApprovalSession::new(self)
    }

    pub(crate) fn sign_user_in(&mut self, jwt: String, ip: IpAddr) -> Result<Option<ClientUserInformation>, JwtDecodeError> {
        let token = self.sess.decode_jwt(&jwt)?;
        let mut user = match self.user.get_user_mut(token.id()) {
            Some(v) => v,
            None => return Ok( None )
        };

        user.add_to_history(UserHistoryElement::new(ip, Utc::now()));

        Ok(
            Some(
                ClientUserInformation::new(
                    user.id(),
                    jwt
                )
            )
        )
    }

    /// Finds a user using a decoded JWT token, and determines if the user is not revoked.
    fn resolve_user(&self, jwt: &str) -> Option<CompleteUserInformation<'_>> {
        let jwt = self.sess.decode_jwt(jwt).ok()?;

        self.user.verify_and_fetch_user(&jwt)
    }
}

type AuthManState = Arc<Mutex<Option<AuthManagerState<ChanneledLogger>>>>;

pub struct AuthProvision<'a, L> where L: LoggerBase {
    inner: MutexGuard<'a, Option<AuthManagerState<L>>>
}
impl<'a, L> AsRef<AuthManagerState<L>> for AuthProvision<'a, L> where L: LoggerBase {
    fn as_ref(&self) -> &AuthManagerState<L> {
        self.inner.as_ref().unwrap()
    }
}
impl<'a, L> AsMut<AuthManagerState<L>> for AuthProvision<'a, L> where L: LoggerBase {
    fn as_mut(&mut self) -> &mut AuthManagerState<L> {
        self.inner.as_mut().unwrap()
    }
}
impl<'a, L> AuthProvision<'a, L> where L: LoggerBase {
    fn new(inner: MutexGuard<'a, Option<AuthManagerState<L>>>) -> Self {
        Self {
            inner
        }
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
            state: Arc::new(Mutex::new(None)),
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
        let mut guard = self.state.lock().await;

        *guard = Some(core)
    }

    #[allow(clippy::await_holding_lock)]
    pub async fn save(&self) -> Result<(), std::io::Error> {
        let guard = self.state.lock().await;

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

    pub async fn get_provision(&self) -> AuthProvision<'_, ChanneledLogger> {
        let guard = self.state.lock().await;
        AuthProvision::new(guard)
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