use tokio::{
    select,
    task::JoinError,
    net::{UnixStream, UnixListener},
    sync::mpsc::{Receiver, Sender, channel},
    fs::{create_dir_all, try_exists, remove_file}
};

use common::{log_debug, log_error, log_info, log_warning, msg::decode_request, task_util::{poll, shutdown, ArgSimplexTask, KillMessage, TaskBasis}};
use regisd_com::{msg::ConsoleRequests, loc::{SERVER_COMM_PATH, SERVER_COMM_DIR}};

use crate::message::{ConsoleComm, WorkerTaskResult};

async fn establish_listener() -> Result<UnixListener, WorkerTaskResult> {
    match try_exists(SERVER_COMM_PATH).await {
        Ok(exists) => {
            if exists {
                remove_file(SERVER_COMM_PATH).await.map_err(|_| WorkerTaskResult::Sockets)?;
            }
        },
        Err(e) => {
            log_error!("(Console) Unable to determine location of the socket file '{e}'");
            return Err(WorkerTaskResult::Sockets);
        }
    }

    if create_dir_all(SERVER_COMM_DIR).await.is_err() {
        return Err(WorkerTaskResult::DoNotReboot);
    }

    let listener = match UnixListener::bind(SERVER_COMM_PATH) {
        Ok(v) => v,
        Err(e) => {
            log_error!("(Console) Unable to open connection to server comm, '{e}'");
            return Err(WorkerTaskResult::Sockets)
        }
    };

    Ok(listener)
}

pub async fn client_worker(mut conn: Receiver<ConsoleComm>, (mut source, send): (UnixStream, Sender<ConsoleComm>)) {
    let to_send: ConsoleComm;

    loop {
        select! {
            v = conn.recv() => {
                match v {
                    Some(v) => match v {
                        ConsoleComm::Poll | ConsoleComm::ReloadConfiguration => continue,
                        ConsoleComm::SystemShutdown | ConsoleComm::Kill => return
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

                log_debug!("(Console Worker) Got request from client '{msg:?}'");

                to_send = match msg {
                    ConsoleRequests::Auth => todo!(),
                    ConsoleRequests::Config => ConsoleComm::ReloadConfiguration,
                    ConsoleRequests::Shutdown => ConsoleComm::SystemShutdown
                };

                break;
            }
        }
    }

    if let Err(e) = send.send(to_send).await {
        log_error!("(Console Worker) Unable to send message to console manager ('{e}'). Aborting...");
    }
}

pub async fn close_tasks<T>(tasks: Vec<T>) -> Vec<Result<Option<T::Output>, JoinError>> where T: TaskBasis, T::Msg: KillMessage {
    let mut result = Vec::with_capacity(tasks.len());
    for task in tasks {
        result.push(
            shutdown(task).await
        )
    }

    result
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
    log_info!("(Console) Listener started.");

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
                                log_info!("(Console) Poll of task {i} determined it was dead.");
                                was_dead += 1;
                                continue;
                            }

                            let current = poll(&mut task).await;

                            
                            if !current {
                                log_warning!("(Console) Poll of task {i} failed.");
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
                    ConsoleComm::ReloadConfiguration => {
                        log_info!("(Console) Configuration reloaded");
                        continue;
                    }
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
                        match v {
                            ConsoleComm::ReloadConfiguration => {
                                log_info!("(Console) Got reload config message from worker thread. Sending to orch...");
                                if let Err(e) = orch.send(ConsoleComm::ReloadConfiguration).await {
                                    log_error!("(Console) Unable to send reload config message to the orch '{e}'");
                                    result_status = WorkerTaskResult::Failure;
                                    break;
                                }
                            },
                            ConsoleComm::Poll => continue,
                            ConsoleComm::Kill | ConsoleComm::SystemShutdown => {
                                log_info!("(Console) Got system shutdown message from worker thread. Sending to orch...");
                                if let Err(e) = orch.send(ConsoleComm::SystemShutdown).await {
                                    log_error!("(Console) Unable to send kill message to the orch '{e}'");
                                    result_status = WorkerTaskResult::Failure;
                                    break;
                                }
                            }
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

    log_info!("(Console) Closing down tasks.");

    let result = close_tasks(active).await;
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
