use common::msg::{MessageBasis, RequestMessage};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ConsoleRequests {
    Shutdown,
    Auth,
    Config
}
impl MessageBasis for ConsoleRequests { }
impl RequestMessage for ConsoleRequests { }