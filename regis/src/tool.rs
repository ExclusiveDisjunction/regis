use std::net::IpAddr;
use std::net::SocketAddr;
use std::net::TcpStream;

use common::log_critical;
use common::metric::BinaryNumber;
use common::metric::Utilization;
pub use common::msg::{send_message, send_request, send_response, decode_message, decode_request, decode_response, Acknoledgement, RequestMessages, ResponseMessages, ServerStatusResponse, MetricsResponse, SendError};

use crate::config::CONFIG;

pub fn connect(host: IpAddr) -> Result<TcpStream, std::io::Error> {
    let port = match CONFIG.access().access() {
        Some(v) => v.port,
        None => {
            log_critical!("Unable to get port from configuration.");
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "the configuration could not be read"));
        }
    };

    println!("Attempting to connect to {} on port {}", &host, port);

    let stream = TcpStream::connect(SocketAddr::from( (host, port) ))?;

    Ok(stream)
}

pub const METRICS_HOLDING: usize = 60;

pub struct SummaryEntry {
    pub time: i64,
    pub cpu_usage: Utilization,
    pub mem_usage: BinaryNumber,
    pub proc_count: u64
}