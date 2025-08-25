pub mod collect;
pub mod io;
pub mod storage;

use collect::collect_all_snapshots;
use io::METRICS;

use exdisj::{log_info, log_debug, log_warning};
use exdisj::io::lock::OptionRwProvider;
use exdisj::io::log::ChanneledLogger;
use exdisj::task::{ChildComm, TaskMessage};
use tokio::select;
use tokio::time::interval;

use std::time::Duration;

use crate::{config::CONFIG, msg::{SimpleComm, WorkerTaskResult}};

pub async fn metrics_entry(logger: ChanneledLogger, mut recv: ChildComm<SimpleComm>) -> WorkerTaskResult {
    let mut freq = match CONFIG.access().access() {
        Some(v) => v.metric_freq,
        None => return WorkerTaskResult::Configuration
    };

    log_info!(&logger, "Started recording with frequency {freq} seconds.");

    let mut intv = interval(Duration::from_secs(freq));

    loop {
        select! {
            v = recv.recv() => {
                match v {
                    TaskMessage::Poll => continue,
                    TaskMessage::Kill => {
                        log_info!(&logger, "Got kill message from Orch.");
                        break;
                    }
                    TaskMessage::Inner(SimpleComm::ReloadConfiguration) => {
                        freq = match CONFIG.access().access() {
                            Some(v) => v.metric_freq,
                            None => {
                                log_warning!(&logger, "Unable to reload from configuration. Aboriting.");
                                return WorkerTaskResult::Configuration;
                            }
                        };
                        intv = interval(Duration::from_secs(freq));
                        log_info!(&logger, "Configuration reloaded");
                        continue;
                    }
                }
            },
            _ = intv.tick() => {
                log_debug!(&logger, "Collecting metrics.");
                let metrics = collect_all_snapshots().await;
                if !METRICS.push(metrics) {
                    log_warning!(&logger, "Unable to insert into metrics. Resetting provider...");
                    METRICS.reset();
                }

                //log_debug!("(Metrics) Metrics inserted");
            }
        }
    }

    log_info!(&logger, "Exiting task, result 'Ok'");
    WorkerTaskResult::Ok
}
