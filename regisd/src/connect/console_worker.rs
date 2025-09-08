use tokio::{
    select,
    net::UnixStream,
    sync::mpsc::Sender
};

use exdisj::{
    io::{
        lock::OptionRwProvider, log::ChanneledLogger, msg::{decode_message_async, send_message_async}
    }, log_debug, log_error, task::{ChildComm, TaskMessage}
};
use common::{
    config::Configuration, msg::{ConsoleAuthRequests, ConsoleConfigRequests, ConsoleFlatRequests, ConsoleRequests, UserDetails, UserSummary}
};

use crate::{auth::man::AUTH, config::CONFIG, msg::ConsoleComm};

/// Represents the actual tasks carried out by connected consoles.
pub async fn console_worker(logger: ChanneledLogger, mut comm: ChildComm<()>, mut source: UnixStream, sender: Sender<ConsoleComm>) {
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

                let flat = msg.flatten();
                match msg {
                    ConsoleRequests::Poll => {
                        if let Err(e) = send_message_async((), &mut source).await {
                            log_error!(&logger, "Unable to send ok message back to console connection: '{e}'.");
                            return;
                        }
                    },
                    ConsoleRequests::Shutdown | ConsoleRequests::Config(ConsoleConfigRequests::Reload) => {
                        let top_request = if flat == ConsoleFlatRequests::Shutdown {
                            ConsoleComm::Shutdown
                        } else {
                            ConsoleComm::ConfigReload(true)
                        };

                        if let Err(e) = sender.send(top_request).await {
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
                                Ok(v) => {
                                    log_debug!(&logger, "Got configuration value. Is some? {}", config.access().is_some());
                                    v
                                }
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
                    ConsoleRequests::Config(ConsoleConfigRequests::Set(new_config)) => {
                        CONFIG.direct_set(new_config);
                        if let Err(e) = sender.send(ConsoleComm::ConfigReload(false)).await {
                            log_error!(&logger, "Unable to send message to console manager: '{e}'.");
                            return;
                        }

                        if let Err(e) = send_message_async(true, &mut source).await {
                            log_error!(&logger, "Unable to send back result of configuration set to console connection '{e:?}'.");
                            return;
                        }
                    },
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