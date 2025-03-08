use std::net::{Ipv4Addr, TcpListener};
use std::net::SocketAddr;

use tokio::sync::mpsc::Receiver;
use tokio::net::{TcpSocket, TcpStream};

use crate::message::{SimpleComm, WorkerTaskResult};
use crate::task_util::{SimplexTask, TaskBasis};
use crate::config::CONFIG;
use crate::log_error;

pub async fn client_entry(_recv: Receiver<SimpleComm>) -> WorkerTaskResult {
    //Establish TCP listener
    let port = match CONFIG.access().access() {
        Some(v) => v.hosts_port,
        None => {
            log_error!("(Clients) Unable to retrive configuration. Exiting task.");
            return WorkerTaskResult::Configuration;
        }
    };

    let addr = SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), port));
    let _listener = TcpListener::bind(addr);

    WorkerTaskResult::Ok
}