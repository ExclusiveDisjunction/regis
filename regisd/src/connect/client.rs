use std::net::{Ipv4Addr, SocketAddr};

use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::sync::mpsc::Receiver;

use common::task_util::{poll, shutdown_tasks, ArgSimplexTask, KillMessage, PollableMessage, TaskBasis};
use common::{log_error, log_info, log_debug};

use crate::config::CONFIG;
use crate::msg::{SimpleComm, WorkerTaskResult};

pub enum WorkerComm {
    Kill,
    Poll
}
impl PollableMessage for WorkerComm {
    fn poll() -> Self {
        Self::Poll
    }
}
impl KillMessage for WorkerComm {
    fn kill() -> Self {
        Self::Kill
    }
}

async fn setup_listener(addr: Ipv4Addr, port: &mut u16, max_clients: &mut usize, old_listener: Option<&mut TcpListener>) -> Result<Option<TcpListener>, WorkerTaskResult> {
    let old_port = *port;
    (*port, *max_clients) = match CONFIG.access().access() {
        Some(v) => (v.hosts_port, v.max_hosts as usize),
        None => {
            log_error!("(Clients) Unable to retrive configuration. Exiting task.");
            return Err(WorkerTaskResult::Configuration);
        }
    };

    log_debug!("(Client) Setting up listener: Old Port: {old_port}, New Port: {}", *port);
    if old_port != *port {
        log_info!("(Client) Listener being reset, opening on port {}", *port);
        let addr = SocketAddr::from((addr, *port));

        let new_listener = match TcpListener::bind(addr).await {
            Ok(v) => v,
            Err(e) => {
                log_error!("(Clients) Unable to open TCP listener '{e}', exiting task.");
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
        log_info!("(Client) Request was made to reset listener, but port did not change. Ignoring update.");
        Ok(None)
    }
}

pub async fn client_entry(mut recv: Receiver<SimpleComm>) -> WorkerTaskResult {
    log_info!("(Client) Starting listener...");

    let mut port: u16 = 0;
    let mut max_clients: usize = 0;
    let addr = Ipv4Addr::new(0, 0, 0, 0);
    let mut listener: TcpListener = match setup_listener(addr.clone(), &mut port, &mut max_clients, None).await {
        Ok(v) => v.expect("It didnt give me the listener, when I expected it!"),
        Err(e) => return e
    };

    log_debug!("(Client) Listener started.");

    let mut result_status: WorkerTaskResult = WorkerTaskResult::Ok;

    let mut active: Vec<ArgSimplexTask<WorkerComm, (), (TcpStream, SocketAddr)>> = vec![];
    loop {
        select! {
            conn = listener.accept() => {
                let conn = match conn {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!("(Clients) Unable to accept from listener '{e}', exiting task.");
                        result_status = WorkerTaskResult::Sockets;
                        break;
                    }
                };

                log_info!("Got connection from '{}'", &conn.1);
                if active.len() >= max_clients {
                    //Send message stating that the network is busy.
                    log_info!("(Clients) Closing connection to '{}' because the max hosts has been reached.", &conn.1);

                    continue;
                }

                log_info!("(Clients) started connection from '{}'", &conn.1);
                active.push(
                    ArgSimplexTask::start(client_worker, 10, conn)
                );
            },
            m = recv.recv() => {
                let m = match m {
                    Some(v) => v,
                    None => {
                        log_error!("(Clients) Unable to receive message from Orch, exiting task.");
                        result_status = WorkerTaskResult::Failure;
                        break;
                    }
                };

                match m {
                    SimpleComm::Poll => {
                        let mut result: bool = true;
                        let old_size = active.len();

                        let mut new_active = Vec::with_capacity(old_size);
                        let mut was_dead: usize = 0;

                        log_info!("(Client) Poll started...");
                        for (i, mut task) in active.into_iter().enumerate() {
                            if !task.is_running() {
                                log_debug!("(Client) Poll of task {i} determined it was dead.");
                                was_dead += 1;
                                continue;
                            }

                            let current = poll(&mut task).await;

                            if !current {
                                log_debug!("(Client) Poll of task {i} failed.");
                                result = false;
                                continue;
                            }

                            log_debug!("(Client) Poll of task {i} passed.");
                            new_active.push(task);
                        }

                        active = new_active;
                        log_info!("(Client) Poll completed, pass? '{}' (dead: {was_dead}, failed: {})", result, old_size - active.len() - was_dead);
                    }
                    SimpleComm::ReloadConfiguration => {
                        if let Err(e) = setup_listener(addr.clone(), &mut port, &mut max_clients, Some(&mut listener)).await {
                            log_error!("(Client) Unable to reload configuration due to error '{e}'");
                            result_status = e;
                            break;
                        }

                        log_info!("(Clients) Configuration reloaded.");
                    }
                    SimpleComm::Kill => {
                        log_info!("(Client) Got shutdown message from Orch.");
                        break;
                    }
                }
            }
        }
    }

    log_debug!("(Client) Closing down tasks.");

    let result = shutdown_tasks(active).await;
    let total = result.len();
    let ok = result.into_iter()
        .map(|x| {
            match x {
                Ok(v) => v.is_some(),
                Err(e) => !e.is_panic()
            }
        })
        .fold(0usize, |acc, x| if x { acc + 1 } else { acc } );

    log_info!("(Client) {ok}/{total} tasks joined with non-panic errors.");
    log_info!("(Client) Exiting task, result '{}'", &result_status);

    result_status
}

pub async fn client_worker(_conn: Receiver<WorkerComm>, (_stream, _source): (TcpStream, SocketAddr)) {}
