use std::net::{Ipv4Addr, SocketAddr};

use common::msg::{RequestMessages, ResponseMessages, ServerStatusResponse, MetricsResponse};
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
use rsa_ext::RsaPublicKey;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;

use crate::auth::man::AUTH;
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
                        client_worker(their_logger, comm, conn.0).await 
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

pub async fn client_worker(logger: ChanneledLogger, mut comm: ChildComm<()>, mut stream: TcpStream) {
    let auth = AUTH.get().unwrap();
    let rng = &mut *auth.get_rng().await;

    // Send the RSA public key.
    let (pub_key, priv_key) = auth.get_rsa().clone().split();
    log_debug!(&logger, "Serializing the RSA public key for the client");
    let pub_key_as_bytes = match serde_json::to_vec(pub_key.public_key()) {
        Ok(v) => v,
        Err(e) => {
            log_error!(&logger, "Unable to serialize the RSA public key, error '{e:?}'");
            return;
        }
    };

    log_debug!(&logger, "Sending the public key to the client.");
    if let Err(e) = send_buffer_async(&pub_key_as_bytes, &mut stream).await {
        log_error!(&logger, "Unable to send the public RSA key '{e:?}'");
        return;
    }

    log_debug!(&logger, "Send complete, waiting for client RSA key.");
    let mut client_rsa_bytes: Vec<u8> = vec![];
    if let Err(e) = receive_buffer_async(&mut client_rsa_bytes, &mut stream).await {
        log_error!(&logger, "Unable to receive the bytes for the client RSA key, error '{e:?}'.");
        return;
    }

    let client_rsa: RsaPublicKey = match serde_json::from_slice(&client_rsa_bytes) {
        Ok(v) => v,
        Err(e) => {
            log_error!(&logger, "Unable to deserialize the client's public RSA key, error: '{e:?}'.");
            return;
        }
    };
    log_debug!(&logger, "Got client RSA key, switching to RSA encrypted channel.");

    let complete_rsa = RsaHandler::from_parts(client_rsa, priv_key.into_inner());
    let mut rsa_stream = RsaStream::new(stream, complete_rsa);

    log_debug!(&logger, "Generating an AES key and sending it to the client over RSA stream.");

    let aes_key = AesHandler::new(rng);
    if let Err(e) = rsa_stream.send_bytes_async(aes_key.as_bytes(), rng).await {
        log_error!(&logger, "Unable to send the AES key over the RSA stream, error '{e:?}'");
        return;
    }

    // Now we can use AES encryption streams
    log_debug!(&logger, "Switching to AES encrypted stream");
    let mut aes_stream = AesStream::new(rsa_stream.take().0, aes_key);

    // TODO: Handle authentication later

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
                if let Err(e) = aes_stream.send_serialize_async(&response, rng).await {
                    log_error!(&logger, "Unable to send message to client '{e}'.");
                    return;
                }
            }
        }
    }
}
