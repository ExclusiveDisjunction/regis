use tokio::{
    select, 
    signal::unix::{
        signal, 
        SignalKind
    },
    sync::mpsc::{Sender, Receiver}
};

use std::time::Duration;
use std::process::ExitCode;

use crate::{
    config::CONFIG, connect::{
        client::mgnt::client_entry, 
        console::mgnt::console_entry
    }, locations::CONFIG_PATH, log_critical, log_debug, log_error, log_info, log_warning, metric::mgnt::metrics_entry
};
use crate::{
    message::{ConsoleComm, SimpleComm},
    task_util::{SimplexTask, DuplexTask, TaskBasis}
};

/// The amount of time between each task "poll". 
pub const TASK_CHECK_TIMEOUT: u64 = 40;



pub struct Orchestrator {
    client_thread: SimplexTask<SimpleComm>,
    metric_thread: SimplexTask<SimpleComm>,
    console_thread: DuplexTask<ConsoleComm>
}
impl Orchestrator {
    pub fn initialize() -> Self{
        let client_thread = SimplexTask::start(client_entry);
        let console_thread = DuplexTask::start(console_entry);
        let metric_thread = SimplexTask::start(metrics_entry);

        Self {
            client_thread,
            console_thread,
            metric_thread
        }
    }

    /// Spawns a timer thread, used to establish some sort of needed task over time.
    pub fn spawn_timer() -> DuplexTask<()> {
        let function = async |(sender, mut receiver): (Sender<()>, Receiver<()>)| -> () {
            loop {
                select! {
                    _ = tokio::time::sleep(Duration::from_secs(TASK_CHECK_TIMEOUT)) => {
                        if sender.send(()).await.is_err() {
                            return;
                        }
                    },
                    _ = receiver.recv() => {
                        return; //Getting anything is considering a close, error or not.
                    }
                }
            }
        };

        DuplexTask::<()>::start(function)
    }

    /// Will spawn the needed timing and shutdown tasks, and will conduct polls & restart tasks as needed.
    pub async fn run(mut self) -> Result<(), ExitCode> {
        // This thread will do polls to determine if threads need to be spanwed. 

        //This needs a timer thread. At a periodic time, this timer will signal this main thread to send out poll requests. It only works with the unit type. If it receives a message, it will immediatley exit. 
        log_info!("Spawning timer & SIGTERM threads...");
        let mut timer_handle = Self::spawn_timer();

        // To handle SIGTERM, since this is a daemon, a separate task needs to be made 
        let mut term_signal = match signal(SignalKind::terminate()) {
            Ok(v) => v,
            Err(e) => panic!("Unable to generate sigterm signal handler '{e}'")
        };
        log_info!("Utility tasks loaded.");

        // Critical code
        loop {
            select! {
                p = timer_handle.recv() => {
                    log_debug!("Timer tick activated");
                    match p {
                        Some(_) => {
                            log_debug!("(Orch) Beginning polls...");
                            let mut result: bool = true;

                            result &= self.client_thread.poll_and_restart(client_entry).await;
                            result &= self.metric_thread.poll_and_restart(metrics_entry).await;
                            result &= self.console_thread.poll_and_restart(console_entry).await;
                            
                            if !result {
                                log_debug!("(Orch) Polls complete, failure.");
                                log_critical!("Unable to restart one or more worker threads, shutting down threads.");
                                break;
                            }

                            log_debug!("(Orch) Polls complete, success.");
                        },
                        None => break
                    }
                },
                _ = term_signal.recv() => {
                    log_info!("SIGTERM message from OS received, shutting down threads.");
                    break;
                },
                m = self.console_thread.recv() => {
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
                        if !self.console_thread.restart(console_entry) {
                            log_critical!("Unable to restart console thread. Shutting down tasks...");
                            break;
                        }
                        log_info!("Console thread restarted.");
                    }
                }
            }
        }

        // Shutting down threads
        log_info!("Shutting down timer thread...");
        timer_handle.shutdown_explicit(()).await;
        log_info!("Timer thread shut down.");

        self.shutdown().await;
    
        Ok(())
    }

    pub async fn shutdown(self) {
        log_info!("Shutting down client, console, and metric threads...");
        self.client_thread.shutdown().await;
        self.console_thread.shutdown().await;
        self.metric_thread.shutdown().await;
        log_info!("Threads shut down.");
    }
}