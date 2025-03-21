use tokio::select;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::Receiver;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use exdisj::{log_debug, log_error, log_info, log_warning};
use exdisj::lock::OptionRwProvider;

use crate::config::CONFIG;
use crate::msg::{SimpleComm, WorkerTaskResult};
use crate::metric::io::EVENTS;

async fn setup_listener(addr: IpAddr, port: &mut u16, old_listener: Option<&mut TcpListener>) -> Result<Option<TcpListener>, WorkerTaskResult> {
    let old_port = *port;
    *port = match CONFIG.access().access() {
        Some(v) => v.broadcasts_port,
        None => {
            log_error!("(Broadcast) Unable to retrive configuration. Exiting task.");
            return Err(WorkerTaskResult::Configuration);
        }
    };

    log_debug!("(Broadcast) Setting up listener: Old Port: {old_port}, New Port: {}", *port);
    if old_port != *port {
        log_info!("(Broadcast) Listener being reset, opening on port {}", *port);
        let addr = SocketAddr::from((addr, *port));

        let new_listener = match TcpListener::bind(addr).await {
            Ok(v) => v,
            Err(e) => {
                log_error!("(Broadcast) Unable to open TCP listener '{e}', exiting task.");
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
        log_info!("(Broadcast) Request was made to reset listener, but port did not change. Ignoring update.");
        Ok(None)
    }
}

pub async fn broadcast_entry(mut recv: Receiver<SimpleComm>) -> Result<(), WorkerTaskResult> {
    log_info!("(Broadcast) Starting listener");

    let mut port: u16 = 0;
    let addr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
    let mut listener = setup_listener(addr, &mut port, None).await?.expect("didn't get listener when it was supposed to be given.");

    log_debug!("(Broadcast) Listener started.");

    let mut sub = EVENTS.subscribe();

    let mut result_status: Result<_, WorkerTaskResult> = Ok(());

    let mut active: Vec<(TcpStream, SocketAddr)> = vec![];
    loop {
        select! {
            conn = listener.accept() => {
                let (socket, address) = match conn {
                    Ok(v) => v,
                    Err(e) => {
                        log_warning!("(Broadcast) Unable to take in connection. Reason '{e}'");
                        continue;
                    }
                };

                log_info!("(Broadcast) Accepted connection from '{}'", address);

                active.push((socket, address));
            },
            val = sub.recv() => {
                let metric = match val {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!("(Broadcast) Unable to get metric from holder, exiting '{e}'.");

                        result_status = Err(WorkerTaskResult::Failure);
                        break;
                    }
                };

                log_debug!("(Broadcast) Got metric from metrics holder");
                let new_streams = Vec::with_capacity(active.len());
                for (mut stream, address) in active {
                    todo!()
                }

                active = new_streams;
            },
            msg = recv.recv() => {
                let msg = match msg {
                    Some(v) => v,
                    None => {
                        log_error!("(Broadcast) Unable to decode message from Orch, aborting.");
                        result_status = Err(WorkerTaskResult::Failure);
                        break;
                    }
                };

                match msg {
                    SimpleComm::Poll => continue,
                    SimpleComm::Kill => break,
                    SimpleComm::ReloadConfiguration => {
                        if let Err(e) = setup_listener(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), &mut port, Some(&mut listener)).await {
                            log_error!("(Broadcast) Unable to reload configuration '{e}'. Exiting");
                            result_status = Err(WorkerTaskResult::Failure);
                            break;
                        }
                    }
                }
            }
        }
    }

    result_status
}