use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
};

use common::{log_error, log_info};

use crate::message::{ConsoleComm, WorkerTaskResult};

pub async fn console_entry(
    (_orch, mut recv): (Sender<ConsoleComm>, Receiver<ConsoleComm>),
) -> WorkerTaskResult {
    loop {
        let v = recv.recv().await;

        log_info!("(Console) Got message from Orch '{:?}'", &v);
        let v = match v {
            Some(v) => v,
            None => {
                log_error!(
                    "(Console) Message could not be received from Orch. Shutting down, exit code 'Failure'."
                );
                return WorkerTaskResult::Failure;
            }
        };
        
        match v {
            ConsoleComm::Poll => continue,
            ConsoleComm::Kill => break,
            ConsoleComm::SystemShutdown => continue,
            ConsoleComm::ReloadConfiguration => {
                log_info!("(Console) Configuration reloaded");
                continue;
            }
        }
    }

    log_info!("(Metrics) Exiting task, result 'Ok'");
    WorkerTaskResult::Ok
}
