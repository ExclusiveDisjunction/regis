use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use common::msg::{MetricsResponse, RequestMessages, ResponseMessages, ServerStatusResponse, SignInMessage, SignInResponse};
use exdisj::{
    log_debug, log_error, log_info, log_warning,
    io::{
        log::{Prefix, ChanneledLogger, ConsoleColor},
        lock::OptionRwProvider,
        net::{receive_buffer_async, send_buffer_async}
    },
    auth::{AesHandler, AesStream, RsaHandler, RsaStream},
    task::{ChildComm, TaskMessage, TaskOnce}
};
use rand::{CryptoRng, RngCore};
use rsa_ext::RsaPublicKey;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;

use crate::auth::{app::ApprovalStatus, man::{AUTH, AuthManager, ClientUserInformation}};
use crate::config::CONFIG;
use crate::metric::collect::collect_all_snapshots;
use crate::metric::io::METRICS;
use crate::msg::{SimpleComm, WorkerTaskResult};

async fn setup_listener(addr: Ipv4Addr, logger: &ChanneledLogger, port: &mut u16, max_clients: &mut usize, old_listener: Option<&mut TcpListener>) -> Result<Option<TcpListener>, WorkerTaskResult> {
    let old_port = *port;
    (*port, *max_clients) = match CONFIG.access().access() {
        Some(v) => (v.hosts_port, v.max_hosts as usize),
        None => {
            log_error!(logger, "Unable to retrive configuration. Exiting task.");
            return Err(WorkerTaskResult::Configuration);
        }
    };

    log_debug!(logger, "Setting up listener: Old Port: {old_port}, New Port: {}", *port);
    if old_port != *port {
        log_info!(logger, "Listener being reset, opening on port {}", *port);
        let addr = SocketAddr::from((addr, *port));

        let new_listener = match TcpListener::bind(addr).await {
            Ok(v) => v,
            Err(e) => {
                log_error!(logger, "Unable to open TCP listener '{e}', exiting task.");
                return Err(WorkerTaskResult::Sockets);
            }
        };

        if let Some(listener) = old_listener {
            *listener = new_listener;
            Ok(None)
        }
        else {
            Ok(Some(new_listener))
        }
    }
    else {
        log_info!(logger, "Request was made to reset listener, but port did not change. Ignoring update.");
        Ok(None)
    }
}

pub async fn client_entry(logger: ChanneledLogger, mut recv: ChildComm<SimpleComm>) -> WorkerTaskResult {
    log_info!(&logger, "Starting listener...");

    let mut port: u16 = 0;
    let mut max_clients: usize = 0;
    let addr = Ipv4Addr::new(0, 0, 0, 0);
    let mut listener: TcpListener = match setup_listener(addr, &logger, &mut port, &mut max_clients, None).await {
        Ok(v) => v.expect("It didnt give me the listener, when I expected it!"),
        Err(e) => return e
    };

    log_debug!(&logger, "Listener started.");

    let mut result_status: WorkerTaskResult = WorkerTaskResult::Ok;

    let mut active: Vec<TaskOnce<(), ()>> = vec![];
    loop {
        select! {
            conn = listener.accept() => {
                let conn = match conn {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!(&logger, "Unable to accept from listener '{e}', exiting task.");
                        result_status = WorkerTaskResult::Sockets;
                        break;
                    }
                };

                if active.len() >= max_clients {
                    //Send message stating that the network is busy.
                    log_info!(&logger, "Closing connection to '{}' because the max hosts has been reached.", &conn.1);

                    continue;
                }

                log_info!(&logger, "Started connection from '{}'", &conn.1);
                let prefix = Prefix::new(format!("Client Worker {}", active.len()), ConsoleColor::Red);
                let their_logger = logger.make_channel(prefix);
                active.push(
                    TaskOnce::new(async move |comm| {
                        client_worker(their_logger, comm, conn.0, conn.1.ip()).await 
                    }, 10, true)
                );
            },
            m = recv.recv() => {
                match m {
                    TaskMessage::Poll => {
                        let mut result: bool = true;
                        let old_size = active.len();

                        let mut new_active = Vec::with_capacity(old_size);
                        let mut was_dead: usize = 0;

                        log_info!(&logger, "Poll started...");
                        for (i, task) in active.into_iter().enumerate() {
                            if !task.poll().await {
                                log_debug!(&logger, "Poll of task {i} failed.");
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
                    TaskMessage::Inner(SimpleComm::ReloadConfiguration) => {
                        if let Err(e) = setup_listener(addr, &logger, &mut port, &mut max_clients, Some(&mut listener)).await {
                            log_error!(&logger, "Unable to reload configuration due to error '{e}'");
                            result_status = e;
                            break;
                        }

                        log_info!(&logger, "Configuration reloaded.");
                    }
                   
                }
            }
        }
    }

    log_info!(&logger, "Closing down tasks.");

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

async fn setup_handshake<R>(logger: &ChanneledLogger, mut stream: TcpStream, auth: &AuthManager, rng: &mut R) -> Option<AesStream<TcpStream>>
where R: CryptoRng + RngCore {
    // Send the RSA public key.
    let (pub_key, priv_key) = auth.get_rsa().clone().split();
    log_debug!(logger, "Serializing the RSA public key for the client");
    let pub_key_as_bytes = match serde_json::to_vec(pub_key.public_key()) {
        Ok(v) => v,
        Err(e) => {
            log_error!(logger, "Unable to serialize the RSA public key, error '{e:?}'");
            return None;
        }
    };

    log_debug!(logger, "Sending the public key to the client.");
    if let Err(e) = send_buffer_async(&pub_key_as_bytes, &mut stream).await {
        log_error!(logger, "Unable to send the public RSA key '{e:?}'");
        return None;
    }

    log_debug!(logger, "Send complete, waiting for client RSA key.");
    let mut client_rsa_bytes: Vec<u8> = vec![];
    if let Err(e) = receive_buffer_async(&mut client_rsa_bytes, &mut stream).await {
        log_error!(logger, "Unable to receive the bytes for the client RSA key, error '{e:?}'.");
        return None;
    }

    let client_rsa: RsaPublicKey = match serde_json::from_slice(&client_rsa_bytes) {
        Ok(v) => v,
        Err(e) => {
            log_error!(logger, "Unable to deserialize the client's public RSA key, error: '{e:?}'.");
            return None;
        }
    };
    log_debug!(logger, "Got client RSA key, switching to RSA encrypted channel.");

    let complete_rsa = RsaHandler::from_parts(client_rsa, priv_key.into_inner());
    let mut rsa_stream = RsaStream::new(stream, complete_rsa);

    log_debug!(logger, "Generating an AES key and sending it to the client over RSA stream.");

    let aes_key = AesHandler::new(rng);
    if let Err(e) = rsa_stream.send_bytes_async(aes_key.as_bytes(), rng).await {
        log_error!(logger, "Unable to send the AES key over the RSA stream, error '{e:?}'");
        return None;
    }

    // Now we can use AES encryption streams
    log_debug!(logger, "Switching to AES encrypted stream");
    let aes_stream = AesStream::new(rsa_stream.take().0, aes_key);

    Some( aes_stream )
}
async fn determine_user_sign_in<R>(logger: &ChanneledLogger, aes_stream: &mut AesStream<TcpStream>, auth: &AuthManager, rng: &mut R, ip: IpAddr) -> Option<ClientUserInformation>
where R: CryptoRng + RngCore {
    let sign_in = match aes_stream.receive_deserialize_async().await {
        Ok(v) => v,
        Err(e) => {
            log_error!(logger, "Unable to decode handshake message from client '{e}'. Exiting");
            return None;
        }
    };

    let mut guard = auth.get_provision().await;
    match sign_in {
        SignInMessage::Returning(jwt) => {
            let manager = guard.as_mut();

            match manager.sign_user_in(jwt, ip) {
                Ok(Some(c)) => {
                    log_info!(logger, "User #{} signed in.", c.id());
                    if let Err(e) = aes_stream.send_serialize_async(&SignInResponse::Approved, rng).await {
                        log_error!(logger, "Unable to send message: '{e}'");
                        return None;
                    }

                    return Some(c)
                },
                Ok(None) => {
                    log_error!(logger, "User could not be found.");
                    let _ = aes_stream.send_serialize_async(&SignInResponse::UserNotFound, rng).await;
                    return None;
                },
                Err(e) => {
                    log_error!(logger, "Unable to decode information: '{e}'.");
                    let _ = aes_stream.send_serialize_async(&SignInResponse::ServerError, rng).await;
                    return None;
                }
            }
        },
        SignInMessage::NewUser => {
            let mut app = guard.as_mut().approvals();
            let status = app.register_request(ip).await;
            match status {
                ApprovalStatus::Approved(v) => {
                    log_info!(logger, "User was approved.");
                    if let Err(e) = aes_stream.send_serialize_async(&SignInResponse::Approved, rng).await {
                        log_error!(logger, "Unable to send message: '{e}'");
                        return None;
                    }

                    return Some(v)
                }
                ApprovalStatus::Denied => {
                    let _ = aes_stream.send_serialize_async(&SignInResponse::Denied, rng).await;
                    log_info!(logger, "User was denied entry. Exiting.");
                    return None;
                }
            }
        }
    }
}

async fn client_worker(logger: ChanneledLogger, mut comm: ChildComm<()>, stream: TcpStream, ip: IpAddr) {
    let auth = AUTH.get().unwrap();
    let mut aes_stream;
    {
        let mut rng_guard = auth.get_rng().await;
        aes_stream = match setup_handshake(&logger, stream, auth, &mut *rng_guard).await {
            Some(v) => v,
            None => return
        };

        let status: ClientUserInformation = match determine_user_sign_in(&logger, &mut aes_stream, auth, &mut *rng_guard, ip).await {
            Some(v) => v,
            None => return
        };
        log_info!(&logger, "Signed In as user ID {}", status.id());
    }

    loop {
        select! {
            v = comm.recv() => {
                match v {
                    TaskMessage::Kill => return,
                    TaskMessage::Poll | TaskMessage::Inner(_) => continue,
                }
            },
            raw_msg = aes_stream.receive_deserialize_async() => {
                let msg: RequestMessages = match raw_msg {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!(&logger, "Unable to decode message from bound client '{e}'. Exiting.");
                        return;
                    }
                };
    
                log_info!(&logger, "Serving request '{:?}'", &msg);
    
                let response: ResponseMessages = match msg {
                    RequestMessages::Metrics(amount) => {
                        let collected = METRICS.view(amount);
    
                        let to_send = if let Some(c) = collected {
                            c
                        }
                        else {
                            log_warning!(&logger, "Unable to retrieve metrics. Resetting metrics.");
                            METRICS.reset();
                            vec![]
                        };
    
                        MetricsResponse { info: to_send }.into()
                    },
                    RequestMessages::Status => {
                        let metrics = collect_all_snapshots().await;
                        ServerStatusResponse { info: metrics }.into()
                    }
                };
    
                log_debug!(&logger, "Sending response message...");
                let mut rng_guard = auth.get_rng().await;
                if let Err(e) = aes_stream.send_serialize_async(&response, &mut *rng_guard).await {
                    log_error!(&logger, "Unable to send message to client '{e}'.");
                    return;
                }
            }
        }
    }
}
