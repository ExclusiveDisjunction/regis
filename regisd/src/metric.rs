pub mod collect;
pub mod io;
pub mod storage;

use common::log_info;
use tokio::select;
use tokio::sync::mpsc::Receiver;

use crate::message::{SimpleComm, WorkerTaskResult};

pub async fn metrics_entry(mut recv: Receiver<SimpleComm>) -> WorkerTaskResult {
    loop {
        select! {
            v = recv.recv() => {
                let v = match v {
                    Some(v) => v,
                    None => return WorkerTaskResult::Failure
                };

                match v {
                    SimpleComm::Poll => continue,
                    SimpleComm::Kill => {
                        log_info!("(Metrics) Got kill message from Orch.");
                        break;
                    }
                    SimpleComm::ReloadConfiguration => {
                        log_info!("(Metrics) Configuration reloaded");
                        continue;
                    }
                }
            }
        }
    }

    log_info!("(Metrics) Exiting task, result 'Ok'");
    WorkerTaskResult::Ok
}
