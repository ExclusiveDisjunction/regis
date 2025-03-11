use common::msg::{MessageBasis, RequestMessage};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ShutdownRequest;
impl MessageBasis for ShutdownRequest {}
impl RequestMessage for ShutdownRequest {}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AuthenticateRequest;
impl MessageBasis for AuthenticateRequest {}
impl RequestMessage for AuthenticateRequest {}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct UpdateConfigRequest;
impl MessageBasis for UpdateConfigRequest {}
impl RequestMessage for UpdateConfigRequest {}