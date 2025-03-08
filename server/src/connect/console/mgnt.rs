use tokio::sync::mpsc::{Sender, Receiver};

use crate::message::{ConsoleComm, WorkerTaskResult};

pub async fn console_entry((_orch, _recv): (Sender<ConsoleComm>, Receiver<ConsoleComm>)) -> WorkerTaskResult{
    todo!()
}