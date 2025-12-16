use std::fmt::Display;

use crate::{config::DaemonConfig, msg::{ConsoleAuthRequests, ConsoleConfigRequests, ConsoleRequests}};
use exdisj::{
    io::log::{ConstructableLogger, Logger}, log_debug, log_error, log_info, log_warning, task::{ChildComm, ShutdownError, TaskMessage, TaskOnce}
};
use serde::{Deserialize, Serialize};

use super::conn::{Connection, ConnectionError};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Default, clap::Args)]
pub struct DaemonConfigUpdate {
    /// The maximum number of console connections allowed.
    #[arg(long = "mconsole")]
    pub max_console: Option<u8>,
    /// THe maximum number of client connections allowed.
    #[arg(long = "mclient")]
    pub max_hosts: Option<u8>,
    /// The port used for client connections.
    #[arg(long = "port")]
    pub hosts_port: Option<u16>,
    /// In seconds, how frequently the system records metrics.
    #[arg(long = "freq")]
    pub metric_freq: Option<u64>
}

#[derive(Clone, Debug)]
pub enum BackendRequests {
    Poll,
    Shutdown,
    Auth(ConsoleAuthRequests),
    ReloadConfig,
    GetConfig,
    UpdateConfig(DaemonConfigUpdate)
}

#[derive(Debug)]
pub enum BackendMessage {
    Req(BackendRequests),
    Resp(Result<Vec<u8>, ConnectionError>)
}
impl From<BackendRequests> for BackendMessage {
    fn from(value: BackendRequests) -> Self {
        Self::Req(value)
    }
}
impl From<Result<Vec<u8>, ConnectionError>> for BackendMessage {
    fn from(value: Result<Vec<u8>, ConnectionError>) -> Self {
        Self::Resp(value)
    }
}
impl BackendMessage {
    pub fn as_request(self) -> Option<BackendRequests> {
        match self {
            Self::Req(v) => Some(v),
            Self::Resp(_) => None
        }
    }
    pub fn as_response(self) -> Option<Result<Vec<u8>, ConnectionError>> {
        match self {
            Self::Req(_) => None,
            Self::Resp(r) => Some(r)
        }
    }
}

pub enum BackendOutput {
    Ok,
    CommFailure
}

pub async fn process_request<L>(msg: BackendRequests, logger: &L, stream: &mut Connection) -> Result<Vec<u8>, ConnectionError> 
where L: Logger + 'static {
    let request: ConsoleRequests = match msg {
        BackendRequests::Poll => ConsoleRequests::Poll,
        BackendRequests::Shutdown => ConsoleRequests::Shutdown,
        BackendRequests::ReloadConfig => ConsoleRequests::Config(ConsoleConfigRequests::Reload),
        BackendRequests::Auth(v) => ConsoleRequests::Auth(v),
        BackendRequests::GetConfig => ConsoleRequests::Config(ConsoleConfigRequests::Get),
        BackendRequests::UpdateConfig(config_diff) => {
            // We must collect the previous metrics, make the changes, and then respond.
            let config_message = stream.send_with_response_bytes(
                ConsoleRequests::Config(ConsoleConfigRequests::Get)
            ).await?;
            let mut config: DaemonConfig = match serde_json::from_slice(&config_message) {
                Ok(v) => v,
                Err(e) => return Err( ConnectionError::Serde(e) )
            };
           
            if let Some(max_console) = config_diff.max_console {
                config.max_console = max_console;
            }
            if let Some(max_hosts) = config_diff.max_hosts {
                config.max_hosts = max_hosts;
            }
            if let Some(hosts_port) = config_diff.hosts_port {
                config.hosts_port = hosts_port;
            }
            if let Some(metric_freq) = config_diff.metric_freq {
                config.metric_freq = metric_freq;
            }

            // Now send back the previous config.
            ConsoleRequests::Config(ConsoleConfigRequests::Set(config))
        }
    };
    log_debug!(logger, "Sending request {:?} to regisd", &request);
    stream.send_with_response_bytes(request).await
}

pub async fn runtime_entry<L>(logger: L, mut comm: ChildComm<BackendMessage>, mut stream: Connection) -> BackendOutput 
where L: Logger + 'static {
    log_info!(&logger, "Begining backend tasks.");

    let mut result = BackendOutput::Ok;

    loop {
        match comm.recv().await {
            TaskMessage::Kill => {
                log_info!(&logger, "The backend was asked to exit.");
                break;
            }
            TaskMessage::Poll => continue,
            TaskMessage::Inner(msg) => {
                if let Some(req) = msg.as_request() {
                    log_info!(&logger, "Processing request {:?}", &req);
                    match process_request(req, &logger, &mut stream).await {
                        Ok(resp) => {
                            if !comm.force_send(Ok(resp).into()).await {
                                log_error!(&logger, "Unable to send response back to the backend controller. Backend exiting.");
                                result = BackendOutput::CommFailure;
                                break;
                            }
                        },
                        Err(e) => {
                            log_error!(&logger, "Unable to process request with error: '{e:?}'. Backend exiting.");

                            result = BackendOutput::CommFailure;
                            break;
                        }
                    }
                }
                else {
                    log_warning!(&logger, "The frontend sent a response message, but only a request was expected");
                    continue;
                }
            }
        }

    }

    result
}

pub struct Backend<L> {
    task: TaskOnce<BackendMessage, BackendOutput>,
    logger: L
}
impl<L> Backend<L> {
    async fn make_handle(logger: L) -> Result<TaskOnce<BackendMessage, BackendOutput>, std::io::Error> 
    where L: Logger + 'static {
        let stream = Connection::open().await?;
        let task = TaskOnce::new(
            async move |comm| { 
                runtime_entry(logger, comm, stream).await
            },
            20,
            false
        );

        Ok( task )
    }

    pub async fn spawn(logger: L) -> Result<Self, std::io::Error> 
    where L: ConstructableLogger + 'static,
          L::Err: Send + Sync + std::error::Error + 'static {
        let their_logger = logger.make_channel("Regisc Backend".into())
            .map_err(std::io::Error::other)?;

        log_info!(&logger, "Attempting to connect to regisd via the socket file.");
        let task = Self::make_handle(their_logger).await?;

        Ok (
            Self {
                task,
                logger
            }
        )
    }

    pub async fn send(&self, value: BackendRequests) -> bool {
        self.task.send(value.into()).await
    }
    pub async fn recv(&mut self) -> Option<Result<Vec<u8>, ConnectionError>> {
        self.task.recv().await?.as_response()
    }
    pub async fn send_with_response(&mut self, message: BackendRequests) -> Option<Result<Vec<u8>, ConnectionError>> {
        if !self.send(message).await {
            return None
        }

        self.recv().await
    }

    pub async fn shutdown(self, with_timeout: bool) -> Result<BackendOutput, ShutdownError<L>>
    where L: ConstructableLogger + 'static {
        self.task.shutdown(with_timeout, &self.logger).await
    }
}
