pub mod collect;
pub mod io;
pub mod storage;

use collect::collect_all_snapshots;
use io::METRICS;

use common::{log_info, log_warning, log_debug};
use tokio::select;
use tokio::sync::mpsc::Receiver;
use tokio::time::interval;

use std::time::Duration;

use crate::{config::CONFIG, msg::{SimpleComm, WorkerTaskResult}};

pub async fn metrics_entry(mut recv: Receiver<SimpleComm>) -> WorkerTaskResult {
    let mut freq = match CONFIG.access().access() {
        Some(v) => v.metric_freq,
        None => return WorkerTaskResult::Configuration
    };

    log_info!("(Metrics) Started recording with frequency {freq} seconds.");

    let mut intv = interval(Duration::from_secs(freq));

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
                        freq = match CONFIG.access().access() {
                            Some(v) => v.metric_freq,
                            None => return WorkerTaskResult::Configuration
                        };
                        intv = interval(Duration::from_secs(freq));
                        log_info!("(Metrics) Configuration reloaded");
                        continue;
                    }
                }
            },
            _ = intv.tick() => {
                let metrics = collect_all_snapshots().await;
                log_debug!("(Metrics) Inserting: '{:?}'", &metrics);
                if !METRICS.push(metrics) {
                    log_warning!("(Metrics) Unable to insert into metrics. Resetting provider...");
                    METRICS.reset();
                }

                log_debug!("(Metrics) Metrics inserted");
            }
        }
    }

    log_info!("(Metrics) Exiting task, result 'Ok'");
    WorkerTaskResult::Ok
}
