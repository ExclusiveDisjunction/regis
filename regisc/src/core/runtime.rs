use exdisj::{log_debug, log_info, log_error};
use exdisj::{io::log::ChanneledLogger, task::{ChildComm, TaskOnce}};
use exdisj::io::msg::{send_request_async, decode_request_async};
use tokio::net::UnixStream;

pub enum RuntimeMessage {

}
pub enum RuntimeOutput {
    InvalidStream,

}

pub async fn runtime_entry(logger: ChanneledLogger, comm: ChildComm<RuntimeMessage>, mut stream: UnixStream) {
    log_info!(&logger, "Begining runtime tasks.");
}

pub struct Runtime {
    task: TaskOnce<RuntimeMessage, RuntimeOutput>,
    logger: ChanneledLogger
}