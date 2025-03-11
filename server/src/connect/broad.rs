use std::net::{Ipv4Addr, SocketAddr};

use common::{log_error, log_info};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::select;
use tokio::sync::mpsc::Receiver;

use crate::locations::BROADCAST_PORT;
use crate::message::{SimpleComm, WorkerTaskResult};

pub async fn broad_entry(mut recv: Receiver<SimpleComm>) -> WorkerTaskResult {
    let mut active: Vec<(TcpStream, TcpSocket)> = Vec::new();

    let addr = SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), BROADCAST_PORT));
    let listener = match TcpListener::bind(addr).await {
        Ok(v) => v,
        Err(_) => return WorkerTaskResult::Sockets,
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
