use std::fs;
use std::os::unix::fs::PermissionsExt;

use tokio::{
    select,
    net::UnixListener,
    sync::mpsc::channel,
    fs::{try_exists, remove_file}
};

use exdisj::{
    io::{
        log::{ConstructableLogger, Logger}
    }, log_debug, log_error, log_info, task::{ChildComm, TaskMessage, TaskOnce}
};
use common::{
    loc::COMM_PATH
};

use crate::msg::{ConsoleComm, WorkerTaskResult};
use super::console_worker::console_worker;

/// Sets up, and tests the connection to the UNIX socket used for communication.
async fn establish_listener(logger: &impl Logger) -> Result<UnixListener, WorkerTaskResult> {
    match try_exists(COMM_PATH).await {
        Ok(exists) => {
            if exists {
                remove_file(COMM_PATH).await.map_err(|_| WorkerTaskResult::Sockets)?;
            }
        },
        Err(e) => {
            log_error!(logger, "Unable to determine location of the socket file '{e}'");
            return Err(WorkerTaskResult::Sockets);
        }
    }

    let listener = match UnixListener::bind(COMM_PATH) {
        Ok(v) => v,
        Err(e) => {
            log_error!(logger, "Unable to open connection to server comm, '{e}'");
            return Err(WorkerTaskResult::Sockets)
        }
    };
    if let Err(e) = fs::set_permissions(COMM_PATH, fs::Permissions::from_mode(0o660)) {
        log_error!(logger, "Unable to set permissions for the server communication: '{e}'");
        return Err(WorkerTaskResult::Sockets);
    }

    Ok(listener)
}

pub async fn console_entry(logger: impl ConstructableLogger + 'static, mut comm: ChildComm<ConsoleComm>) -> WorkerTaskResult {
    log_info!(&logger, "Starting listener...");
    let listener = match establish_listener(&logger, ).await {
        Ok(v) => v,
        Err(e) => {
            log_error!(&logger, "Unable to start listener. Aborting.");
            return e
        }
    };
    log_debug!(&logger, "Listener started.");

    let (send, mut worker_recv) = channel::<ConsoleComm>(5);
    let mut active: Vec<TaskOnce<(), ()>> = vec![];

    let mut result_status = WorkerTaskResult::Ok;
    loop {
        select! {
            v = comm.recv() => {
                match v {
                    TaskMessage::Poll => {
                        let mut result: bool = true;
                        let old_size = active.len();

                        let mut new_active = Vec::with_capacity(old_size);
                        let mut was_dead: usize = 0;

                        log_info!(&logger, "Poll started...");
                        for (i, task) in active.into_iter().enumerate() {
                            if !task.poll().await {
                                log_debug!(&logger, "Poll of task {i} determined it was dead.");
                                was_dead += 1;
                                result = false;
                                continue;
                            }

                            log_debug!(&logger, "Poll of task {i} passed.");

                            new_active.push(task);
                        }

                        active = new_active;
                        log_info!(&logger, "Poll completed, pass? '{}' (dead: {was_dead}, failed: {})", result, old_size - active.len() - was_dead);
                    }
                    TaskMessage::Kill => {
                        log_info!(&logger, "Got shutdown message from Orch.");
                        break;
                    }
                    TaskMessage::Inner(_) => continue
                }
            },
            conn = listener.accept() => {
                let conn = match conn {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!(&logger, "Stream could not be accepted from UnixListener '{}'.", e);
                        result_status = WorkerTaskResult::Failure;
                        break;
                    }
                };

                log_info!(&logger, "Accepted connection from '{:?}'", &conn.1);

                let their_logger = match logger.make_channel( format!("Console Worker {}", active.len()).into() ) {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!(&logger, "Unable to create a logger for the console worker: '{e:?}'. Aborting.");
                        continue;
                    }
                };
                let their_sender = send.clone();
                active.push(
                    TaskOnce::new(async move |comm| {
                        console_worker(their_logger, comm, conn.0, their_sender).await
                    }, 5, true)
                )
            },
            msg = worker_recv.recv() => {
                match msg {
                    Some(v) => {
                        log_info!(&logger, "Got message '{:?}' from worker thread.", v);
                        
                        if !comm.force_send(v).await {
                            log_error!(&logger, "Unable to send message to the orch.");
                            result_status = WorkerTaskResult::Failure;
                            break;
                        }
                    }
                    None => {
                        log_error!(&logger, "Worker receiver could not get message.");
                        result_status = WorkerTaskResult::Failure;
                        break;
                    }
                }
            }
        }
    }

    log_debug!(&logger, "Closing down tasks.");

    let mut result = Vec::with_capacity(active.len());
    for task in active {
        result.push(task.shutdown(true, &logger).await);
    }
    let total = result.len();
    let ok = result.into_iter()
        .map(|x| {
            match x {
                Ok(_) => true,
                Err(e) => !e.is_fatality()
            }
        })
        .fold(0usize, |acc, x| if x { acc + 1 } else { acc } );

    log_info!(&logger, "{ok}/{total} tasks joined with non-panic errors.");

    log_info!(&logger, "Exiting task, result '{}'", &result_status);
    result_status
}
