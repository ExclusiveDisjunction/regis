use tokio::{
    select,
    signal::unix::{SignalKind, signal},
    task::JoinError,
    time::{Duration, interval},
};

use std::process::ExitCode;

use crate::{
    config::CONFIG,
    connect::{client::client_entry, console::console_entry},
    loc::CONFIG_PATH,
    msg::{ConsoleComm, SimpleComm, WorkerTaskResult},
    metric::metrics_entry,
};

use common::{
    log_critical, log_debug, log_error, log_info, log_warning,
    task_util::{DuplexTask, RestartableTask, SimplexTask, recv, shutdown},
};

/// The amount of time between each task "poll".
pub const TASK_CHECK_TIMEOUT: u64 = 30;
/// The buffer size for a default channel buffer.
pub const TASKS_DEFAULT_BUFFER: usize = 10;

pub struct Orchestrator {
    client_thread: RestartableTask<SimplexTask<SimpleComm, WorkerTaskResult>>,
    metric_thread: RestartableTask<SimplexTask<SimpleComm, WorkerTaskResult>>,
    console_thread: RestartableTask<DuplexTask<ConsoleComm, WorkerTaskResult>>
}
impl Orchestrator {
    pub fn initialize() -> Self {
        let client_thread = RestartableTask::start(client_entry, TASKS_DEFAULT_BUFFER, 5);
        let console_thread = RestartableTask::start(console_entry, TASKS_DEFAULT_BUFFER, 5);
        let metric_thread = RestartableTask::start(metrics_entry, TASKS_DEFAULT_BUFFER, 5);

        Self {
            client_thread,
            console_thread,
            metric_thread,
        }
    }

    /// Will spawn the needed timing and shutdown tasks, and will conduct polls & restart tasks as needed.
    pub async fn run(mut self) -> Result<(), ExitCode> {
        // This thread will do polls to determine if threads need to be spanwed.

        //This needs a timer thread. At a periodic time, this timer will signal this main thread to send out poll requests. It only works with the unit type. If it receives a message, it will immediatley exit.
        log_info!("(Orch) Spawning timer & SIGTERM threads...");
        let mut timer = interval(Duration::from_secs(TASK_CHECK_TIMEOUT));

        // To handle SIGTERM, since this is a daemon, a separate task needs to be made
        let mut term_signal = match signal(SignalKind::terminate()) {
            Ok(v) => v,
            Err(e) => panic!("(Orch) Unable to generate sigterm signal handler '{e}'"),
        };
        let mut sig_int = match signal(SignalKind::interrupt()) {
            Ok(v) => v,
            Err(e) => panic!("(Orch) Unable to generate a sigint signal handler '{e}'"),
        };
        log_info!("(Orch) Utility tasks loaded.");

        // Critical code
        loop {
            select! {
                _ = timer.tick() => {
                    log_debug!("(Orch) Timer tick activated");
                    log_info!("(Orch) Beginning polls...");
                    let mut result: bool = true;

                    result &= self.client_thread.poll_and_restart(client_entry, TASKS_DEFAULT_BUFFER).await.log_event("client");
                    result &= self.metric_thread.poll_and_restart(metrics_entry, TASKS_DEFAULT_BUFFER).await.log_event("metrics");
                    result &= self.console_thread.poll_and_restart(console_entry, TASKS_DEFAULT_BUFFER).await.log_event("console");

                    if !result {
                        log_info!("(Orch) Polls complete, failure.");
                        log_critical!("Unable to restart one or more worker threads, shutting down all threads.");
                        break;
                    }

                    log_info!("(Orch) Polls complete, success.");
                },
                _ = term_signal.recv() => {
                    log_info!("(Orch) SIGTERM message from OS received, shutting down threads.");
                    break;
                },
                _ = sig_int.recv() => {
                    log_info!("(Orch) SIGINT message from OS received, shutting down threads.");
                    break;
                }
                m = recv(&mut self.console_thread) => {
                    if let Some(m) = m {
                        match m {
                            ConsoleComm::ReloadConfiguration => {
                                log_info!("(Orch) By request of console thread, the configuration is being reloaded...");
                                if let Err(e) = CONFIG.open(CONFIG_PATH) {
                                    log_error!("(Orch) Unable to reload configuration due to '{:?}'. Configuration will be reset to defaults.", e);
                                    CONFIG.set_to_default();
                                }
                            },
                            ConsoleComm::SystemShutdown => {
                                log_info!("(Orch) By request of console thread, the system is to shutdown...");
                                log_info!("(Orch) Shutting down threads...");
                                break;
                            },
                            v => log_warning!("(Orch) Got innapropriate request from console thread: '{v}'. Ignoring.")
                        }
                    }
                    else {
                        log_info!("(Orch) Console thread unexpectedly closed. Attempting to restart...");
                        if self.console_thread.restart(console_entry, TASKS_DEFAULT_BUFFER).is_err() {
                            log_critical!("Unable to restart console thread. Shutting down tasks...");
                            break;
                        }
                        log_info!("(Orch) Console thread restarted.");
                    }
                }
            }
        }

        // Shutting down threads
        self.shutdown().await;

        Ok(())
    }

    pub async fn shutdown(self) {
        let transform = |x: Result<Option<WorkerTaskResult>, JoinError>| -> String {
            match x {
                Ok(v) => match v {
                    Some(v) => v.to_string(),
                    None => "Already Joined".to_string(),
                },
                Err(e) => format!("join error: '{e}'"),
            }
        };

        let shutdowns = vec![
            shutdown(self.client_thread).await,
            shutdown(self.console_thread).await,
            shutdown(self.metric_thread).await
        ];
        let mut iter = shutdowns.into_iter()
        .map(transform);

        log_info!(
            "(Orch) Client task shutdown with response '{}'",
            iter.next().unwrap()
        );
        log_info!(
            "(Orch) Console task shutdown with response '{}'",
            iter.next().unwrap()
        );
        log_info!(
            "(Orch) Metric task shutdown with response '{}'",
            iter.next().unwrap()
        );
        log_info!("(Orch) Tasks shut down.");
    }
}
