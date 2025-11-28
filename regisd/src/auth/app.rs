use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::task::{Poll, Waker};
use std::net::IpAddr;
use std::sync::Arc;

use tokio::sync::RwLock;

use chrono::{DateTime, Utc};
use common::msg::PendingUser;

use crate::auth::man::ClientUserInformation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalStatus<V> {
    Denied,
    Approved(V)
}

#[derive(Debug)]
pub struct ApprovalRequestCore {
    waker: Option<Waker>,
    status: Option<ApprovalStatus<u64>>, //Some(ApprovalStatus::Approved(id)) -> Approved, Some(ApprovalStatus::Denied) -> Denied, None -> Not determiend
    timeout: DateTime<Utc>
}
impl ApprovalRequestCore {
    pub fn new(waker: Option<Waker>) -> Self {
        Self {
            waker,
            status: None,
            timeout: Utc::now() + std::time::Duration::from_secs(5 * 60) //5 minutes times 60 seconds
        }
    }

    pub fn set_status(&mut self, approved: ApprovalStatus<u64>) {
        self.status = Some(approved);
        if let Some(waker) = self.waker.as_ref() {
            waker.wake_by_ref();
        }
    }
    pub fn set_approved(&mut self, new_id: u64) {
        self.set_status(ApprovalStatus::Approved(new_id));
    }
    pub fn set_denied(&mut self) {
        self.set_status(ApprovalStatus::Denied);
    }
    pub fn is_timeout(&self) -> bool {
        Utc::now() >= self.timeout
    }
}

#[derive(Debug)]
pub struct ApprovalRequest {
    user: PendingUser,
    core: Option<Arc<RwLock<ApprovalRequestCore>>>
}
impl Deref for ApprovalRequest {
    type Target = PendingUser;
    fn deref(&self) -> &Self::Target {
        &self.user
    }
}
impl DerefMut for ApprovalRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.user
    }
}
impl ApprovalRequest {
    pub fn new(user: PendingUser) -> Self {
        Self {
            user,
            core: None
        }
    }

    pub(super) fn with_async(&mut self, fut: &ApprovalRequestFuture) {
        self.core = Some(fut.core.clone())
    }
}

pub struct ApprovalRequestFuture {
    user: PendingUser,
    core: Arc<RwLock<ApprovalRequestCore>>
}
impl ApprovalRequestFuture {
    pub(super) fn new(user: PendingUser) -> Self {
        Self {
            user,
            core: Arc::new(
                RwLock::new(
                    ApprovalRequestCore::new(None)
                )
            )
        }
    }

    pub(super) fn with_waker(&mut self, waker: Waker) {
        let mut guard = self.core.blocking_write();
        guard.waker = Some(waker)
    }
}
impl std::future::Future for ApprovalRequestFuture {
    type Output = ApprovalStatus<ClientUserInformation>;
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        // See if the internal state is already completed.
        {
            let read_guard = self.core.blocking_read();
            if let Some(status) = read_guard.status {
                return match status {
                    ApprovalStatus::Approved(new_id) => {
                        todo!("We need to grab out of the manager provision for id {new_id}")
                    },
                    ApprovalStatus::Denied => Poll::Ready(ApprovalStatus::Denied)
                };
            }
        }

        //Otherwise, we are not ready so we give the core a waker, and return incomplete.
        {
            let mut write_guard = self.core.blocking_write();
            write_guard.waker = Some(cx.waker().clone())
        }

        Poll::Pending
    }
}

#[derive(Debug)]
pub(crate) struct ApprovalsManager {
    current_id: u64,
    pending: HashMap<u64, ApprovalRequest>
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
    pub(crate) fn pending(&self) -> Vec<&PendingUser> {
        self.pending.values().map(|x| x.deref() ).collect()
    }
    pub(super) fn contains_pending(&self, id: u64) -> bool {
        self.pending.contains_key(&id)
    }
    pub(super) fn register_request(&mut self, from_ip: IpAddr) -> &mut ApprovalRequest {
        let id = self.current_id;
        self.current_id += 1;

        let user = PendingUser::new(id, from_ip, Utc::now());
        let pending = ApprovalRequest::new(user);
        self.pending.insert(id, pending);

        self.pending.get_mut(&id).unwrap()
    }
    pub(super) fn approve_user(&mut self, with_id: u64, new_id: u64) -> Option<PendingUser> {
        let info = self.pending.remove(&with_id)?;
        if let Some(inner) = info.core {
            let mut guard = inner.blocking_write();
            guard.set_approved(new_id);
        }

        Some( info.user )
    }
    pub(super) fn deny_user(&mut self, with_id: u64) -> bool {
        let info = match self.pending.remove(&with_id) {
            Some(v) => v,
            None => return false
        };

        if let Some(inner) = info.core {
            let mut guard = inner.blocking_write();
            guard.set_denied();
        }

        true
    }
}