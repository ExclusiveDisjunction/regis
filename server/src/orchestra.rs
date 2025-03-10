use tokio::{
    select, 
    signal::unix::{
        signal, 
        SignalKind
    },
    task::JoinError, 
    time::{
        interval,
        Duration}
};

use std::process::ExitCode;

use crate::{
    config::CONFIG, connect::{
        broad::broad_entry, client::client_entry, console::console_entry
    }, locations::CONFIG_PATH, message::{
        ConsoleComm,
        SimpleComm,
        WorkerTaskResult
    }, metric::metrics_entry
};

use common:: {
    log_critical, 
    log_error,
    log_info,
    log_warning,
    log_debug,
    task_util::{
        SimplexTask, 
        DuplexTask,
        RestartableTask, 
        recv, 
        shutdown
    }
};

/// The amount of time between each task "poll". 
pub const TASK_CHECK_TIMEOUT: u64 = 40;
/// The buffer size for a default channel buffer. 
pub const TASKS_DEFAULT_BUFFER: usize = 10;


pub struct Orchestrator {
    client_thread: RestartableTask<SimplexTask<SimpleComm, WorkerTaskResult>>,
    metric_thread: RestartableTask<SimplexTask<SimpleComm, WorkerTaskResult>>,
    console_thread: RestartableTask<DuplexTask<ConsoleComm, WorkerTaskResult>>,
    broad_thread: RestartableTask<SimplexTask<SimpleComm, WorkerTaskResult>>
}
impl Orchestrator {
    pub fn initialize() -> Self{
        let client_thread = RestartableTask::start(client_entry,TASKS_DEFAULT_BUFFER, 5);
        let console_thread = RestartableTask::start(console_entry, TASKS_DEFAULT_BUFFER, 5);
        let metric_thread = RestartableTask::start(metrics_entry, TASKS_DEFAULT_BUFFER, 5);
        let broad_thread = RestartableTask::start(broad_entry, TASKS_DEFAULT_BUFFER, 5);

        Self {
            client_thread,
            console_thread,
            metric_thread,
            broad_thread
        }
    }

    /// Will spawn the needed timing and shutdown tasks, and will conduct polls & restart tasks as needed.
    pub async fn run(mut self) -> Result<(), ExitCode> {
        // This thread will do polls to determine if threads need to be spanwed. 

        //This needs a timer thread. At a periodic time, this timer will signal this main thread to send out poll requests. It only works with the unit type. If it receives a message, it will immediatley exit. 
        log_info!("Spawning timer & SIGTERM threads...");
        let mut timer = interval(Duration::from_secs(TASK_CHECK_TIMEOUT));

        // To handle SIGTERM, since this is a daemon, a separate task needs to be made 
        let mut term_signal = match signal(SignalKind::terminate()) {
            Ok(v) => v,
            Err(e) => panic!("Unable to generate sigterm signal handler '{e}'")
        };
        log_info!("Utility tasks loaded.");

        // Critical code
        loop {
            select! {
                _ = timer.tick() => {
                    log_debug!("Timer tick activated");
                    log_debug!("(Orch) Beginning polls...");
                    let mut result: bool = true;

                    result &= self.client_thread.poll_and_restart(client_entry, TASKS_DEFAULT_BUFFER).await;
                    result &= self.metric_thread.poll_and_restart(metrics_entry, TASKS_DEFAULT_BUFFER).await;
                    result &= self.console_thread.poll_and_restart(console_entry, TASKS_DEFAULT_BUFFER).await;
                    result &= self.broad_thread.poll_and_restart(broad_entry, TASKS_DEFAULT_BUFFER).await;
                    
                    if !result {
                        log_debug!("(Orch) Polls complete, failure.");
                        log_critical!("Unable to restart one or more worker threads, shutting down threads.");
                        break;
                    }

                    log_debug!("(Orch) Polls complete, success.");
                },
                _ = term_signal.recv() => {
                    log_info!("SIGTERM message from OS received, shutting down threads.");
                    break;
                },
                m = recv(&mut self.console_thread) => {
                    if let Some(m) = m {
                        match m {
                            ConsoleComm::ReloadConfiguration => {
                                log_info!("By request of console thread, the configuration is being reloaded...");
                                if let Err(e) = CONFIG.open(CONFIG_PATH) {
                                    log_error!("Unable to reload configuration due to '{:?}'. Configuration will be reset to defaults.", e);
                                    CONFIG.set_to_default();
                                }
                            },
                            ConsoleComm::SystemShutdown => {
                                log_info!("By request of console thread, the system is to shutdown...");
                                log_info!("Shutting down threads...");
                                break;
                            },
                            v => log_warning!("Got innapropriate request from console thread: '{v}'. Ignoring.")
                        }
                    }
                    else {
                        log_info!("Console thread unexpectedly closed. Attempting to restart...");
                        if !self.console_thread.restart(console_entry, TASKS_DEFAULT_BUFFER) {
                            log_critical!("Unable to restart console thread. Shutting down tasks...");
                            break;
                        }
                        log_info!("Console thread restarted.");
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
                Ok(v) => {
                    match v {
                        Some(v) => v.to_string(),
                        None => "Already Joined".to_string()
                    }
                },
                Err(e) => format!("join error: '{e}'")
            }
        };

        log_info!("Client task shutdown with response '{}'", transform(shutdown(self.client_thread).await));
        log_info!("Console task shutdown with response '{}'", transform(shutdown(self.console_thread).await));
        log_info!("Metric task shutdown with response '{}'", transform(shutdown(self.metric_thread).await));
        log_info!("Broadcast task shutdown with response '{}'", transform(shutdown(self.broad_thread).await));
        log_info!("Tasks shut down."); 
    }
}