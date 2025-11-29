use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AuthRequests {
    FirstTime,
    Returning(String)
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum DenialReason {
    Revoked,
    Invalid,
    UserNotFound
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AuthResponses {
    Deny(DenialReason),
    /// Accepted from the first time request
    Accepted(String),
    /// Accepted from the returning. 
    Approved
}