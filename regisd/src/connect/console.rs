use std::fs;
use std::os::unix::fs::PermissionsExt;

use tokio::{
    select,
    net::{UnixStream, UnixListener},
    sync::mpsc::{Sender, channel},
    fs::{create_dir_all, try_exists, remove_file}
};

use exdisj::{
    io::{
        lock::OptionRwProvider, log::{ChanneledLogger, ConsoleColor, Prefix}, msg::{decode_message_async, send_message_async}
    }, log_debug, log_error, log_info, log_warning, task::{ChildComm, TaskMessage, TaskOnce}
};
use common::{
    loc::{COMM_PATH, TOTAL_DIR}, msg::{ConsoleAuthRequests, ConsoleConfigRequests, ConsoleRequests, UserDetails, UserSummary}
};

use crate::{auth::man::AUTH, config::{Configuration, CONFIG}, msg::{ConsoleComm, WorkerTaskResult}};

/// Sets up, and tests the connection to the UNIX socket used for communication.
async fn establish_listener(logger: &ChanneledLogger) -> Result<UnixListener, WorkerTaskResult> {
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

    if create_dir_all(TOTAL_DIR).await.is_err() {
        return Err(WorkerTaskResult::DoNotReboot);
    }

    let listener = match UnixListener::bind(COMM_PATH) {
        Ok(v) => v,
        Err(e) => {
            log_error!(logger, "Unable to open connection to server comm, '{e}'");
            return Err(WorkerTaskResult::Sockets)
        }
    };

    if let Err(e) = fs::set_permissions(COMM_PATH, fs::Permissions::from_mode(0o777)) {
        log_error!(logger, "Unable to set permissions for the server communication, '{e}'");
        return Err(WorkerTaskResult::Sockets);
    }

    Ok(listener)
}

/// Represents the actual tasks carried out by connected consoles.
pub async fn console_worker(logger: ChanneledLogger, mut comm: ChildComm<()>, mut source: UnixStream, sender: Sender<ConsoleRequests>) {
    let auth = AUTH.get().expect("Auth is not initalized");

    loop {
        select! {
            v = comm.recv() => { //Something from parent Console
                match v {
                    TaskMessage::Poll | TaskMessage::Inner(_) => continue,
                    TaskMessage::Kill => return
                }
            }
            raw_msg = decode_message_async(&mut source) => {
                let msg: ConsoleRequests = match raw_msg {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!(&logger, "Unable to decode message from bound client '{e}'");
                        return;
                    }
                };

                log_debug!(&logger, "Processing request '{:?}' from console connection", &msg);

                match msg {
                    ConsoleRequests::Poll | ConsoleRequests::Shutdown | ConsoleRequests::Config(ConsoleConfigRequests::Reload) => {
                        if let Err(e) = sender.send(msg).await {
                            log_error!(&logger, "Unable to send message to console manager: '{e}'.");
                            return;
                        }

                        if let Err(e) = send_message_async((), &mut source).await {
                            log_error!(&logger, "Unable to send ok message back to console connection: '{e}'.");
                            return;
                        }
                    },
                    ConsoleRequests::Config(ConsoleConfigRequests::Get) => {
                        let result = {
                            let config = CONFIG.access();
                            match serde_json::to_vec(&config.access()) {
                                Ok(v) => v,
                                Err(e) => {
                                    log_error!(&logger, "Unable to serialize the value (error: '{e:?}', returning None.");
                                    serde_json::to_vec::<Option<&Configuration>>(&None).expect("unable to serialize none???")
                                }
                            };
                        };

                        if let Err(e) = send_message_async(result, &mut source).await {
                            log_error!(&logger, "Unable to send ok message back to console connection '{e:?}'.");
                            return;
                        }
                    },
                    ConsoleRequests::Config(ConsoleConfigRequests::Set) => todo!(),
                    ConsoleRequests::Auth(v) => {
                        match v {
                            ConsoleAuthRequests::AllUsers => {
                                let result: Vec<UserSummary> = {
                                    let auth_provision = auth.get_provision();

                                    auth_provision.get_all_users()
                                        .into_iter()
                                        .map(|user| UserSummary::new(user.id(), user.nickname().to_string()))
                                        .collect()
                                };

                                if let Err(e) = send_message_async(result, &mut source).await {
                                    log_error!(&logger, "Unable to send ok message back to console connection: '{e}'.");
                                    return;
                                };
                            },
                            ConsoleAuthRequests::UserHistory(id) => {
                                let result = {
                                    let auth_provision = auth.get_provision();

                                    auth_provision.get_user_info(id).map(|user| 
                                        UserDetails::new(
                                            user.id(),
                                            user.nickname().to_string(),
                                            user.history().to_vec() 
                                        )
                                    );
                                };

                                if let Err(e) = send_message_async(result, &mut source).await {
                                    log_error!(&logger, "Unable to send ok message back to console connection: '{e}'.");
                                    return;
                                }
                            },
                            ConsoleAuthRequests::Pending => todo!(),
                            ConsoleAuthRequests::Revoke(id) => {
                                todo!("revoke the user with id {id}")
                            }
                            ConsoleAuthRequests::Approve(id) => {
                                todo!("approve the authorization with id {id}")
                            }
                        };
                    }
                };
            }
        }
    };
}

pub async fn console_entry(logger: ChanneledLogger, mut comm: ChildComm<ConsoleComm>) -> WorkerTaskResult {
    log_info!(&logger, "Starting listener...");
    let listener = match establish_listener(&logger, ).await {
        Ok(v) => v,
        Err(e) => {
            log_error!(&logger, "Unable to start listener. Aborting.");
            return e
        }
    };
    log_debug!(&logger, "Listener started.");

    let (send, mut worker_recv) = channel::<ConsoleRequests>(5);
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

                let prefix = Prefix::new(format!("Console Worker {}", active.len()), ConsoleColor::Yellow);
                let their_logger = logger.make_channel(prefix);
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
                        let to_orch = match v {
                            ConsoleRequests::Shutdown => ConsoleComm::SystemShutdown,
                            ConsoleRequests::Auth(auth) => {
                                todo!("fill out {auth:?}...")
                            },
                            ConsoleRequests::Poll => continue,
                            ConsoleRequests::Config(ConsoleConfigRequests::Reload) => ConsoleComm::ReloadConfiguration,
                            x => {
                                log_warning!(&logger, "Got message '{x:?}' from console worker, ignoring.");
                                continue;
                            }
                        };
                        log_info!(&logger, "Sending {to_orch} to orch");
                        
                        if !comm.force_send(to_orch).await {
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
