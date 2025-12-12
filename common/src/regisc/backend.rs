use std::fmt::Display;

use crate::{config::DaemonConfig, msg::{ConsoleAuthRequests, ConsoleConfigRequests, ConsoleRequests}};
use exdisj::{
    io::log::{ConstructableLogger, Logger}, log_debug, log_error, log_info, log_warning, task::{ChildComm, ShutdownError, TaskMessage, TaskOnce}
};

use super::conn::{Connection, ConnectionError};

#[derive(Clone, Debug)]
pub enum BackendRequests {
    Poll,
    Shutdown,
    Auth(ConsoleAuthRequests),
    ReloadConfig,
    GetConfig,
    UpdateConfig(DaemonConfig)
}

#[derive(Clone, Debug)]
pub enum BackendError {

}
impl Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Nothing so far...")
    }
}

#[derive(Clone, Debug)]
pub enum BackendMessage {
    Req(BackendRequests),
    Resp(Result<Vec<u8>, BackendError>)
}
impl From<BackendRequests> for BackendMessage {
    fn from(value: BackendRequests) -> Self {
        Self::Req(value)
    }
}
impl From<Result<Vec<u8>, BackendError>> for BackendMessage {
    fn from(value: Result<Vec<u8>, BackendError>) -> Self {
        Self::Resp(value)
    }
}
impl From<BackendError> for BackendMessage {
    fn from(value: BackendError) -> Self {
        Self::Resp(Err(value))
    }
}
impl BackendMessage {
    pub fn as_request(self) -> Option<BackendRequests> {
        match self {
            Self::Req(v) => Some(v),
            Self::Resp(_) => None
        }
    }
    pub fn as_response(self) -> Option<Result<Vec<u8>, BackendError>> {
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
        BackendRequests::UpdateConfig(config) => ConsoleRequests::Config(ConsoleConfigRequests::Set(config))
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
    pub async fn recv(&mut self) -> Option<Result<Vec<u8>, BackendError>> {
        self.task.recv().await?.as_response()
    }
    pub async fn send_with_response(&mut self, message: BackendRequests) -> Option<Result<Vec<u8>, BackendError>> {
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
