use std::net::IpAddr;
use std::net::SocketAddr;
use std::net::TcpStream;

use exdisj::io::log::LoggerBase;
use exdisj::io::lock::OptionRwProvider;
use exdisj::log_critical;
use common::metric::BinaryNumber;
use common::metric::Utilization;
pub use common::msg::{RequestMessages, ResponseMessages, ServerStatusResponse, MetricsResponse};
//use exdisj::io::msg::{Acknoledgement, SendError};

use crate::config::CONFIG;

pub fn connect<L>(host: IpAddr, logger: &L) -> Result<TcpStream, std::io::Error> where L: LoggerBase {
    let port = match CONFIG.access().access() {
        Some(v) => v.port,
        None => {
            log_critical!(logger, "Unable to get port from configuration.");
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "the configuration could not be read"));
        }
    };

    println!("Attempting to connect to {} on port {}", &host, port);

    TcpStream::connect(SocketAddr::from( (host, port) ))
}

pub const METRICS_HOLDING: usize = 60;

pub struct SummaryEntry {
    pub time: i64,
    pub cpu_usage: Utilization,
    pub mem_usage: BinaryNumber,
    pub proc_count: u64
}