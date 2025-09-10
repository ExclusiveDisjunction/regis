use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::task::{Poll, Waker};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use common::msg::PendingUser;

#[derive(Debug)]
pub struct PendingUserInfo {
    inner: PendingUser,
    waker: Option<Waker>
}
impl Deref for PendingUserInfo {
    type Target = PendingUser;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for PendingUserInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl PendingUserInfo {
    pub fn new(inner: PendingUser) -> Self {
        Self {
            inner,
            waker: None
        }
    }
    pub fn new_waker(inner: PendingUser, waker: Waker) -> Self {
        Self {
            inner,
            waker: Some(waker)
        }
    }

    pub fn into_inner(self) -> PendingUser {
        self.inner
    }

    pub fn register_waker(&mut self, waker: Waker) {
        self.waker = Some(waker)
    }
    pub fn drop_waker(&mut self) -> Option<Waker> {
        let mut new_waker = None;
        std::mem::swap(&mut self.waker, &mut new_waker);

        new_waker
    }

    pub fn call_waker(&self) {
        self.waker.as_ref().map(|x| x.wake_by_ref());
    }
}

#[derive(Debug)]
pub struct ApprovalsManager {
    current_id: u64,
    pending: HashMap<u64, PendingUserInfo>
}
impl Default for ApprovalsManager {
    fn default() -> Self {
        Self {
            current_id: 0,
            pending: HashMap::new()
        }
    }
}
impl ApprovalsManager {
    fn pending(&self) -> Vec<&PendingUser> {
        self.pending.values().map(|x| x.deref() ).collect()
    }
    fn register_new_user(&mut self, from_ip: IpAddr) {
        let id = self.current_id;
        self.current_id += 1;
        
        let pending = PendingUserInfo::new(PendingUser::new(id, from_ip, Utc::now()));
        self.pending.insert(id, pending);
    }
    fn approve_user(&mut self, with_id: u64) -> Option<PendingUser> {
        let info = self.pending.remove(&with_id)?;
        info.call_waker();

        Some( info.into_inner() )
    }
    fn deny_user(&mut self, with_id: u64) -> bool {
        self.pending.remove(&with_id).is_some()
    }
}

pub struct ApprovalFuture {
    inner: Arc<Mutex<ApprovalsManager>>
}
impl Future for ApprovalFuture {
    type Output = Result<ClientUserInformation, ApprovalError>;
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let mut guard = match self.inner.try_lock() {
            Ok(v) => v,
            Err(e) => return Poll::Pending
        };

        
    }
}

#[derive(Debug)]
pub enum ApprovalError {
    UserNotFound(u64),
    JWT(jwt::Error),
    Denied
}