use serde::{Serialize, Deserialize};

pub use exdisj::msg::*;
use exdisj::metric::PrettyPrinter;

use crate::metric::CollectedMetrics;

use std::fmt::{Display, Debug};

/// General response about server status
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ServerStatusResponse {
    pub info: CollectedMetrics
}
impl MessageBasis for ServerStatusResponse {}
impl ResponseMessage for ServerStatusResponse {} 
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
impl MessageBasis for MetricsResponse {}
impl ResponseMessage for MetricsResponse {}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum RequestMessages {
    Status,
    Metrics(usize),
    Ack(Acknoledgement)
}
impl From<usize> for RequestMessages {
    fn from(value: usize) -> Self {
        Self::Metrics(value)
    }
}
impl From<Acknoledgement> for RequestMessages {
    fn from(value: Acknoledgement) -> Self {
        Self::Ack(value)
    }
}
impl MessageBasis for RequestMessages {}
impl RequestMessage for RequestMessages { }

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ResponseMessages {
    Status(ServerStatusResponse),
    Metrics(MetricsResponse),
    Ack(Acknoledgement)
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
impl From<Acknoledgement> for ResponseMessages {
    fn from(value: Acknoledgement) -> Self {
        Self::Ack(value)
    }
}
impl MessageBasis for ResponseMessages { }
impl ResponseMessage for ResponseMessages { }

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ConsoleRequests {
    Shutdown,
    Auth,
    Config,
    Poll
}
impl MessageBasis for ConsoleRequests { }
impl RequestMessage for ConsoleRequests { }