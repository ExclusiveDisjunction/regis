use std::net::IpAddr;
use std::net::SocketAddr;
use std::io::Error as IOError;
use std::time::Duration;
use tokio::net::TcpStream;

use exdisj::io::log::LoggerBase;
use exdisj::io::lock::OptionRwProvider;
use exdisj::{log_warning, log_critical, log_info};
use common::metric::BinaryNumber;
use common::metric::Utilization;
pub use common::msg::{RequestMessages, ResponseMessages, ServerStatusResponse, MetricsResponse};
//use exdisj::io::msg::{Acknoledgement, SendError};

use common::config::REGIS_CONFIG;

pub async fn connect<L>(host: IpAddr, logger: &L) -> Result<TcpStream, IOError> where L: LoggerBase {
    let port = match REGIS_CONFIG.access().access() {
        Some(v) => v.port,
        None => {
            log_critical!(logger, "Unable to get port from configuration.");
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "the configuration could not be read"));
        }
    };

    println!("Attempting to connect to {} on port {}, with a timeout of 10s", &host, port);

    let timer = tokio::time::sleep(Duration::from_secs(10));
    let address = SocketAddr::from( (host, port) );
    tokio::select! {
        _ = timer => {
            log_warning!(logger, "Timeout reached.");
            Err( IOError::new(std::io::ErrorKind::ConnectionRefused, "the connection could not be made."))
        },
        stream = TcpStream::connect(address) => {
            log_info!(logger, "Connection made.");
            stream
        }
    }
}

pub const METRICS_HOLDING: usize = 60;

pub struct SummaryEntry {
    pub time: i64,
    pub cpu_usage: Utilization,
    pub mem_usage: BinaryNumber,
    pub proc_count: u64
}