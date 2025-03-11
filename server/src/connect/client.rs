use std::net::{Ipv4Addr, SocketAddr};

use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::sync::mpsc::Receiver;

use common::task_util::{ArgSimplexTask, shutdown_explicit};
use common::{log_error, log_info, log_warning};

use crate::config::CONFIG;
use crate::message::{SimpleComm, WorkerTaskResult};

async fn setup_listener(addr: Ipv4Addr) -> Result<(usize, TcpListener), WorkerTaskResult> {
    let (port, max_clients) = match CONFIG.access().access() {
        Some(v) => (v.hosts_port, v.max_hosts as usize),
        None => {
            log_error!("(Clients) Unable to retrive configuration. Exiting task.");
            return Err(WorkerTaskResult::Configuration);
        }
    };

    let addr = SocketAddr::from((addr, port));
    let listener = match TcpListener::bind(addr).await {
        Ok(v) => v,
        Err(e) => {
            log_error!("(Clients) Unable to open the TCP listener '{e}', exiting task.");
            return Err(WorkerTaskResult::Sockets);
        }
    };

    Ok((max_clients, listener))
}

pub async fn client_entry(mut recv: Receiver<SimpleComm>) -> WorkerTaskResult {
    //Establish TCP listener
    let (mut max_clients, mut listener) = match setup_listener(Ipv4Addr::new(0, 0, 0, 0)).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    let mut connected: Vec<ArgSimplexTask<(), (), (TcpStream, SocketAddr)>> = vec![];
    loop {
        select! {
            conn = listener.accept() => {
                let conn = match conn {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!("(Clients) Unable to accept from listener '{e}', exiting task.");
                        return WorkerTaskResult::Sockets;
                    }
                };

                log_info!("Got connection from '{}'", &conn.1);
                if connected.len() >= max_clients {
                    //Send message stating that the network is busy.
                    log_info!("(Clients) Closing connection to '{}' because the max hosts has been reached.", &conn.1);

                    continue;
                }

                log_info!("(Clients) started connection from '{}'", &conn.1);
                connected.push(
                    ArgSimplexTask::start(client_worker, 10, conn)
                );
            },
            m = recv.recv() => {
                let m = match m {
                    Some(v) => v,
                    None => {
                        log_error!("(Clients) Unable to receive message from Orch, shutting down with exit code 'Failure'.");
                        return WorkerTaskResult::Failure;
                    }
                };

                match m {
                    SimpleComm::Poll => continue,
                    SimpleComm::ReloadConfiguration => {
                        (max_clients, listener) = match setup_listener(Ipv4Addr::new(0, 0, 0, 0)).await {
                            Ok(v) => v,
                            Err(e) => {
                                log_error!("(Client) Unable to reload configuration due to error '{}'", &e);
                                return e
                            }
                        };
                        log_info!("(Clients) Configuration reloaded.");
                    }
                    SimpleComm::Kill => {
                        let mut ok = true;
                        for task in connected {
                            if let Err(e) = shutdown_explicit(task, ()).await {
                                log_warning!("(Client) helper task failed to exit properly, reason '{e}'");
                                ok = false;
                            }
                        }

                        if ok {
                            break
                        }
                        else {
                            log_warning!("(Client) One or more helpers failed, shutting down. Exit code 'ImproperShutdown'.");
                            return WorkerTaskResult::ImproperShutdown;
                        }
                    }
                }
            }
        }
    }

    WorkerTaskResult::Ok
}

pub async fn client_worker(_conn: Receiver<()>, (_stream, _source): (TcpStream, SocketAddr)) {}
