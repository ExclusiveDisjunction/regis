use tokio::{
    select,
    net::{UnixStream, UnixListener},
    sync::mpsc::{Receiver, Sender, channel},
    fs::{create_dir_all, try_exists, remove_file}
};

use std::fs;
use std::os::unix::fs::PermissionsExt;

use common::{log_debug, log_error, log_info, msg::decode_request, task_util::{poll, shutdown_tasks, ArgSimplexTask, TaskBasis}};
use regisd_com::{msg::ConsoleRequests, loc::{TOTAL_DIR, COMM_PATH}};

use crate::msg::{ConsoleComm, WorkerTaskResult};

/// Sets up, and tests the connection to the UNIX socket used for communication.
async fn establish_listener() -> Result<UnixListener, WorkerTaskResult> {
    match try_exists(COMM_PATH).await {
        Ok(exists) => {
            if exists {
                remove_file(COMM_PATH).await.map_err(|_| WorkerTaskResult::Sockets)?;
            }
        },
        Err(e) => {
            log_error!("(Console) Unable to determine location of the socket file '{e}'");
            return Err(WorkerTaskResult::Sockets);
        }
    }

    if create_dir_all(TOTAL_DIR).await.is_err() {
        return Err(WorkerTaskResult::DoNotReboot);
    }

    let listener = match UnixListener::bind(COMM_PATH) {
        Ok(v) => v,
        Err(e) => {
            log_error!("(Console) Unable to open connection to server comm, '{e}'");
            return Err(WorkerTaskResult::Sockets)
        }
    };

    if let Err(e) = fs::set_permissions(COMM_PATH, fs::Permissions::from_mode(0o777)) {
        log_error!("(Console) Unable to set permissions for the server communication, '{e}'");
        return Err(WorkerTaskResult::Sockets);
    }

    Ok(listener)
}

/// Represents the actual tasks carried out by connected consoles.
pub async fn client_worker(mut conn: Receiver<ConsoleComm>, (mut source, send): (UnixStream, Sender<ConsoleComm>)) {
    let mut to_send: Option<ConsoleComm> = None;

    loop {
        select! {
            v = conn.recv() => { //Something from parent Console
                match v {
                    Some(v) => match v {
                        ConsoleComm::Poll | ConsoleComm::ReloadConfiguration | ConsoleComm::Auth => {
                            if v == ConsoleComm::Poll {
                                log_info!("(Console Worker) Someone was just saying hi!");
                            }
                        }
                        ConsoleComm::SystemShutdown | ConsoleComm::Kill => break
                    },
                    None => return
                }
            }
            raw_msg = decode_request(&mut source) => {
                let msg: ConsoleRequests = match raw_msg {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!("(Console Worker) Unable to decode message from bound client '{e}'");
                        return;
                    }
                };

                to_send = Some(ConsoleComm::from(msg));

                break;
            }
        }
    }

    if let Some(to_send) = to_send {
        if let Err(e) = send.send(to_send).await {
            log_error!("(Console Worker) Unable to send message to console manager ('{e}')");
        }
    }
}

pub async fn console_entry(
    (orch, mut recv): (Sender<ConsoleComm>, Receiver<ConsoleComm>),
) -> WorkerTaskResult {
    log_info!("(Console) Starting listener...");
    let listener = match establish_listener().await {
        Ok(v) => v,
        Err(e) => {
            log_error!("(Console) Unable to start listener. Aborting.");
            return e
        }
    };
    log_debug!("(Console) Listener started.");

    let (send, mut worker_recv) = channel::<ConsoleComm>(5);
    let mut active: Vec<ArgSimplexTask<ConsoleComm, (), (UnixStream, Sender<ConsoleComm>)>> = vec![];

    let mut result_status = WorkerTaskResult::Ok;
    loop {
        select! {
            v = recv.recv() => {
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
                    ConsoleComm::Poll => {
                        let mut result: bool = true;
                        let old_size = active.len();

                        let mut new_active = Vec::with_capacity(old_size);
                        let mut was_dead: usize = 0;

                        log_info!("(Console) Poll started...");
                        for (i, mut task) in active.into_iter().enumerate() {
                            if !task.is_running() {
                                log_debug!("(Console) Poll of task {i} determined it was dead.");
                                was_dead += 1;
                                continue;
                            }

                            let current = poll(&mut task).await;

                            if !current {
                                log_debug!("(Console) Poll of task {i} failed.");
                                result = false;
                                continue;
                            }

                            log_debug!("(Console) Poll of task {i} passed.");

                            new_active.push(task);
                        }

                        active = new_active;
                        log_info!("(Console) Poll completed, pass? '{}' (dead: {was_dead}, failed: {})", result, old_size - active.len() - was_dead);
                    }
                    ConsoleComm::Kill | ConsoleComm::SystemShutdown => {
                        log_info!("(Console) got shutdown message from Orch.");
                        break;
                    }
                    ConsoleComm::ReloadConfiguration | ConsoleComm::Auth => continue
                }
            },
            conn = listener.accept() => {
                let conn = match conn {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!("(Console) Stream could not be accepted from UnixListener '{}'.", e);
                        result_status = WorkerTaskResult::Failure;
                        break;
                    }
                };

                let their_sender = send.clone();
                log_info!("(Console) Accepted connection from '{:?}'", &conn.1);

                active.push(
                    ArgSimplexTask::start(client_worker, 10, (conn.0, their_sender))
                );
            },
            msg = worker_recv.recv() => {
                match msg {
                    Some(v) => {
                        if matches!(v, ConsoleComm::Poll) {
                            continue;
                        }

                        // The orch will not listen to ConsoleComm::Kill, so we convert it to SystemShutdown.
                        let v = if matches!(v, ConsoleComm::Kill) {
                            ConsoleComm::SystemShutdown
                        }
                        else {
                            v
                        };

                        log_info!("(Console) Got message '{}' from worker thread. Sending to orch.", v);
                        if let Err(e) = orch.send(v).await {
                            log_error!("(Console) Unable to send message to the orch '{e}'.");
                            result_status = WorkerTaskResult::Failure;
                            break;
                        }
                    }
                    None => {
                        log_error!("(Console) Worker receiver could not get message.");
                        result_status = WorkerTaskResult::Failure;
                        break;
                    }
                }
            }
        }
    }

    log_debug!("(Console) Closing down tasks.");

    let result = shutdown_tasks(active).await;
    let total = result.len();
    let ok = result.into_iter()
    .map(|x| {
        match x {
            Ok(v) => v.is_some(),
            Err(e) => !e.is_panic()
        }
    })
    .fold(0usize, |acc, x| if x { acc + 1} else { acc } );
    log_info!("(Console) {ok}/{total} tasks joined with non-panic errors.");

    log_info!("(Console) Exiting task, result '{}'", &result_status);
    result_status
}
