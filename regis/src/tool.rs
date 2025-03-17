use std::collections::HashMap;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::RwLock;

use common::lock::ProtectedAccess;
use common::lock::RwProvider;
use common::lock::RwProviderAccess;
use common::log_critical;
use common::metric::BinaryNumber;
use common::metric::CollectedMetrics;
use common::metric::Utilization;
use common::msg::DecodeError;
pub use common::msg::{send_message, send_request, send_response, decode_message, decode_request, decode_response, Acknoledgement, RequestMessages, ResponseMessages, ServerStatusResponse, MetricsResponse, SendError};
use common::storage::LimitedQueue;

use crate::config::CONFIG;

use lazy_static::lazy_static;

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

pub struct MetricsSummary {
    pub data: HashMap<i64, SummaryEntry>
}


pub struct MetricsProvider {
    data: Arc<RwLock<LimitedQueue<CollectedMetrics>>>
}
impl RwProvider for MetricsProvider {
    type Data = LimitedQueue<CollectedMetrics>;

    fn access_raw(&self) -> common::lock::ProtectedAccess<'_, std::sync::Arc<std::sync::RwLock<Self::Data>>> {
        ProtectedAccess::new(&self.data)
    }
}
impl RwProviderAccess for MetricsProvider { }
impl Default for MetricsProvider {
    fn default() -> Self {
        Self {
            data: Arc::new(RwLock::new(LimitedQueue::new(METRICS_HOLDING)))
        }
    }
}
impl MetricsProvider {
    pub fn push(&self, new: Vec<CollectedMetrics>) {
        let mut lock = self.access_mut();
        let access = match lock.access() {
            Some(v) => v,
            None => {
                self.pass(LimitedQueue::new(METRICS_HOLDING));

                lock = self.access_mut();
                lock.access().expect("Something happened with this storage bruh.")
            }
        };

        let mut to_push: Vec<CollectedMetrics> = vec![];
        
        {
            let mut already_there: HashMap<i64, &mut CollectedMetrics> = HashMap::new();
            let prev = access.get_mut(new.len());
            for item in prev {
                already_there.insert(item.time, item);
            }

            for item in new {
                match already_there.get_mut(&item.time) {
                    Some(v) => **v = item,
                    None => to_push.push(item)
                }
            }
        }

        for new in to_push {
            access.insert(new);
        }
    }

    pub fn request_fresh(&self, connection: &mut TcpStream, amount: usize) -> Result<(), std::io::Error> {
        let request = RequestMessages::Metrics(amount);
        if let Err(e) = send_request(request, connection) {
            match e {
                SendError::IO(io) => return Err(io),
                SendError::Serde(s) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, s))
            }
        }

        let response: ResponseMessages = match decode_response(connection) {
            Ok(v) => v,
            Err(e) => {
                match e {
                    DecodeError::IO(io) => return Err(io),
                    DecodeError::Serde(serde) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, serde)),
                    DecodeError::UTF(utf) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, utf))
                }
            }
        };

        let extracted = match response {
            ResponseMessages::Metrics(v) => v.info,
            _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "should not have gotten anything but the metrics response."))
        };

        self.push(extracted);

        Ok(())
    }

    pub fn summarize(&self, count: usize) -> Option<MetricsSummary> {
        let lock = self.access();

        let access = lock.access()?;
        let points = access.get(count);

        let mut result = HashMap::new();

        

        Some(MetricsSummary { data: result } )
    }

    pub fn clear(&self) {
        let mut lock = self.access_mut();
        
        match lock.access() {
            Some(v) => v.clear(),
            None => self.pass(LimitedQueue::new(METRICS_HOLDING))
        }
    }
}

lazy_static! {
    pub static ref SUMMARY: MetricsProvider = MetricsProvider::default();
}