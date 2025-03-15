use std::net::{Ipv4Addr, SocketAddr};

use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::sync::mpsc::Receiver;

use crate::CONFIG;
use crate::msg::{SimpleComm, WorkerTaskResult};

use common::{log_error, log_info};

async fn setup_listener(addr: Ipv4Addr) -> Result<TcpListener, WorkerTaskResult> {
    let port = match CONFIG.access().access() {
        Some(v) => v.broadcasts_port,
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

    Ok(listener)
}

pub async fn broad_entry(mut recv: Receiver<SimpleComm>) -> WorkerTaskResult {
    let mut active: Vec<(TcpStream, SocketAddr)> = Vec::new();

    let listener = match setup_listener(Ipv4Addr::new(0, 0, 0, 0)).await {
        Ok(l) => l,
        Err(e) => return e
    };

    loop {
        select! {
            conn = listener.accept() => {
                let conn = match conn {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!("(Broadcast) Unable to accept from listener '{e}', exiting task.");
                        return WorkerTaskResult::Sockets;
                    }
                };

                log_info!("(Broadcast) Got connection from '{}'", &conn.1);

                active.push(conn);
            },
            m = recv.recv() => {
                let m = match m {
                    Some(v) => v,
                    None => break
                };

                match m {
                    SimpleComm::Poll => continue,
                    SimpleComm::ReloadConfiguration => continue,
                    SimpleComm::Kill => {
                        active.clear();
                        break;
                    }
                }
            }
        }
    }

    WorkerTaskResult::Ok
}
