use std::os::unix::process;
use std::sync::mpsc::Sender;

use exdisj::task::{ParentComm, ShutdownError, TaskMessage};
use exdisj::{log_debug, log_info, log_error, log_warning};
use exdisj::{io::log::ChanneledLogger, task::{ChildComm, TaskOnce}};
use exdisj::io::msg::{send_request_async, decode_request_async};
use tokio::net::UnixStream;
use tokio::task::JoinError;

use crate::core::conn::{Connection, ConnectionError};

#[derive(Clone, Debug)]
pub enum BackendRequests {
    Poll,
    Shutdown,
    DetermineAuth,
    ReloadConfig,
    GetConfig,
    UpdateConfig
}
#[derive(Clone, Debug)]
pub enum BackendResponses {
    Ok,

}  

#[derive(Clone, Debug)]
pub enum BackendMessage {
    Req(BackendRequests),
    Resp(BackendResponses)
}
impl From<BackendRequests> for BackendMessage {
    fn from(value: BackendRequests) -> Self {
        Self::Req(value)
    }
}
impl From<BackendResponses> for BackendMessage {
    fn from(value: BackendResponses) -> Self {
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
    pub fn as_response(self) -> Option<BackendResponses> {
        match self {
            Self::Req(_) => None,
            Self::Resp(r) => Some(r)
        }
    }
}

pub enum BackendOutput {
    Ok,
    InvalidStream,
    CommFailure
}

pub async fn process_request(msg: BackendRequests, logger: &ChanneledLogger, stream: &mut Connection) -> Result<(), ConnectionError> {
    todo!()
}

pub async fn runtime_entry(logger: ChanneledLogger, mut comm: ChildComm<BackendMessage>, mut stream: Connection) -> BackendOutput {
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
                    if let Err(e) = process_request(req, &logger, &mut stream).await {
                        log_error!(&logger, "Unable to process request with error: '{e:?}'. Backend exiting.");

                        result = BackendOutput::InvalidStream;
                        break;
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

pub struct Backend {
    task: TaskOnce<BackendMessage, BackendOutput>,
    logger: ChanneledLogger
}
impl Backend {
    async fn make_handle(logger: ChanneledLogger) -> Result<TaskOnce<BackendMessage, BackendOutput>, std::io::Error> {
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

    pub async fn spawn(logger: ChanneledLogger) -> Result<Self, std::io::Error> {
        let their_logger = logger.clone();
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
    pub async fn recv(&mut self) -> Option<BackendResponses> {
        self.task.recv().await?.as_response()
    }
    pub async fn send_with_response(&mut self, message: BackendRequests) -> Option<BackendResponses> {
        if !self.send(message).await {
            return None
        }

        self.recv().await
    }

    pub async fn shutdown(self, with_timeout: bool) -> Result<BackendOutput, ShutdownError> {
        self.task.shutdown(with_timeout, &self.logger).await
    }
}