use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use exdisj::io::metric::PrettyPrinter;

use crate::{metric::CollectedMetrics, user::UserHistoryElement};

use std::{fmt::{Debug, Display}, net::IpAddr, ops::Deref};

/// General response about server status
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ServerStatusResponse {
    pub info: CollectedMetrics
}
impl Display for ServerStatusResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.info.pretty_print(0, None)
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MetricsResponse {
    pub info: Vec<CollectedMetrics>
}
impl Display for MetricsResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let each_metric: Vec<String> = self.info.iter()
        .enumerate()
            .map(|(i, x)| x.pretty_print(0, Some(i+1)))
            .collect();

        let joined = each_metric.join("\n");
        
        write!(f, "{joined}")
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum RequestMessages {
    Status,
    Metrics(usize)
}
impl From<usize> for RequestMessages {
    fn from(value: usize) -> Self {
        Self::Metrics(value)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ResponseMessages {
    Status(ServerStatusResponse),
    Metrics(MetricsResponse)
}
impl From<ServerStatusResponse> for ResponseMessages {
    fn from(value: ServerStatusResponse) -> Self {
        Self::Status(value)
    }
}
impl From<MetricsResponse> for ResponseMessages {
    fn from(value: MetricsResponse) -> Self {
        Self::Metrics(value)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct PendingUser {
    id: u64,
    ip: IpAddr,
    time: DateTime<Utc>
}
impl PendingUser {
    pub fn new(id: u64, ip: IpAddr, time: DateTime<Utc>) -> Self {
        Self {
            id,
            ip,
            time
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn ip(&self) -> IpAddr {
        self.ip
    }
    pub fn time(&self) -> DateTime<Utc> {
        self.time
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserSummary {
    id: u64,
    nickname: String
}
impl UserSummary {
    pub fn new(id: u64, nickname: String) -> Self {
        Self {
            id, 
            nickname
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn nickname(&self) -> &str {
        &self.nickname
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserDetails {
    summ: UserSummary,
    history: Vec<UserHistoryElement>
}
impl Deref for UserDetails {
    type Target = UserSummary;
    fn deref(&self) -> &Self::Target {
        &self.summ
    }
}
impl UserDetails {
    pub fn new(id: u64, nickname: String, history: Vec<UserHistoryElement>) -> Self {
        Self {
            summ: UserSummary::new(id, nickname),
            history
        }
    }

    pub fn history(&self) -> &[UserHistoryElement] {
        &self.history
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleAuthRequests {
    Pending,         // Response -> Vec<PendingUser>
    Revoke(u64),     // Response -> bool
    Approve(u64),    // Response -> bool
    AllUsers,        // Response -> Vec<UserSummary>
    UserHistory(u64) // Response -> Vec<UserDetails>
}
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ConsoleConfigRequests {
    Reload, // Response -> ()
    Get,    // Response -> Config
    Set     // Response -> ()
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ConsoleRequests {
    Shutdown,                      // Response -> ()
    Auth(ConsoleAuthRequests),     // Response -> (Depends on request)
    Config(ConsoleConfigRequests), // Response -> (Depends on request)
    Poll                           // Response -> ()
}

#[deprecated]
//#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ConsoleAuthResponses {
    Pending(Vec<PendingUser>),
    AllUsers(Vec<UserSummary>),
    SpecificUser(UserDetails),
    UserNotFound,
    AuthNotFound
}

#[deprecated]
//#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsoleResponses {
    Ok,
    #[allow(deprecated)]
    Auth(ConsoleAuthResponses)
}