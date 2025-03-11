use tokio::{
    select,
    net::{UnixStream, UnixListener},
    sync::mpsc::{Receiver, Sender},
    fs::create_dir_all
};

use common::{log_error, log_info};
use regisd_com::loc::{SERVER_COMM_PATH, SERVER_COMM_DIR};

use crate::message::{ConsoleComm, WorkerTaskResult};

async fn establish_listener() -> Result<UnixListener, WorkerTaskResult> {
    if create_dir_all(SERVER_COMM_DIR).await.is_err() {
        return Err(WorkerTaskResult::DoNotReboot);
    }

    let listener = match UnixListener::bind(SERVER_COMM_PATH) {
        Ok(v) => v,
        Err(_) => return Err(WorkerTaskResult::Sockets)
    };

    Ok(listener)
}

pub async fn console_entry(
    (_orch, mut recv): (Sender<ConsoleComm>, Receiver<ConsoleComm>),
) -> WorkerTaskResult {
    let mut listener = match establish_listener().await {
        Ok(v) => v,
        Err(e) => return e
    };

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
