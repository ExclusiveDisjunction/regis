use std::sync::Arc;

use tokio::{
    select,
    signal::unix::{signal, Signal, SignalKind},
    task::JoinError,
    time::{interval, Duration},
};

use crate::{
    config::CONFIG, 
    connect::{
        client::client_entry, 
        console::console_entry
    }, 
    failure::DaemonFailure, 
    metric::metrics_entry, 
    msg::{
        ConsoleComm, 
        SimpleComm, 
        WorkerTaskResult
    },
    setup::{
        self, 
        Options
    }
};
use common::loc::DAEMON_CONFIG_PATH;

use exdisj::{
    io::{lock::OptionRwProvider, log::{ChanneledLogger, ConsoleColor, Logger, Prefix}}, log_critical, log_error, log_info, log_warning, task::{RestartError, Task}
};

/// The amount of time between each task "poll".
pub const TASK_CHECK_TIMEOUT: u64 = 30;
/// The buffer size for a default channel buffer.
pub const TASKS_DEFAULT_BUFFER: usize = 10;

pub const ORCH_PREFIX: Prefix = Prefix::new_const("Orch", ConsoleColor::Blue);
pub const CONS_PREFIX: Prefix = Prefix::new_const("Console", ConsoleColor::Cyan);
pub const CLNT_PREFIX: Prefix = Prefix::new_const("Client", ConsoleColor::Magenta);
pub const METR_PREFIX: Prefix = Prefix::new_const("Metric", ConsoleColor::Green);

struct SignalBundle {
    term: Signal,
    int: Signal,
    hup: Signal
}

pub struct Orchestrator {
    client: Task<SimpleComm, WorkerTaskResult>,
    metric: Task<SimpleComm, WorkerTaskResult>,
    console: Task<ConsoleComm, WorkerTaskResult>,

    options: Options,
    log: ChanneledLogger
}
impl Orchestrator {
    pub fn initialize(log: &Logger, options: setup::Options) -> Self {
        let my_log = log.make_channel(ORCH_PREFIX.clone());
        let log_arc = Arc::new(my_log);

        let mut client = Task::new(async |comm|{
            let logger = log_arc.make_channel(CLNT_PREFIX.clone());
            client_entry(logger, comm).await
        }, TASKS_DEFAULT_BUFFER, true);

        let mut console = Task::new(async |comm| {
            let logger = my_log.make_channel(CONS_PREFIX.clone());
            console_entry(logger, comm).await
        }, TASKS_DEFAULT_BUFFER, false);

        let mut metric = Task::new(async |comm| {
            let logger = my_log.make_channel(METR_PREFIX.clone());
            metrics_entry(logger, comm).await
        }, TASKS_DEFAULT_BUFFER, true);

        client.with_logger(&my_log);
        console.with_logger(&my_log);
        metric.with_logger(&my_log);

        client.with_restarts(5);
        console.with_restarts(5);
        metric.with_restarts(5);

        Self {
            client,
            console,
            metric,
            options,
            log: my_log
        }
    } 

    fn get_signals() -> Result<SignalBundle, std::io::Error> {
        Ok( 
            SignalBundle {
                term: signal(SignalKind::terminate())?,
                int: signal(SignalKind::interrupt())?,
                hup: signal(SignalKind::hangup())?    
            }
        )
    }

    async fn poll(&mut self) -> bool {
        log_info!(&self.log, "Beginning polls...");
        let mut result: bool = true;

        result &= self.client.poll_and_restart().await.is_ok();
        result &= self.console.poll_and_restart().await.is_ok();
        result &= self.metric.poll_and_restart().await.is_ok();

        if !result {
            log_info!(&self.log, "Polls complete, failure.");
            log_critical!(&self.log, "Unable to restart one or more worker threads, shutting down all threads.");
            return false;
        }

        log_info!(&self.log, "Polls complete, success.");
        true
    }

    async fn reload_configuration(&mut self) -> Result<(), DaemonFailure> {
        if let Err(e) = CONFIG.open(DAEMON_CONFIG_PATH) {
            log_error!(&self.log, "Unable to reload configuration, due to '{:?}'.", e);
            if self.options.override_config {
                log_info!(&self.log, "By request, the configuration will be reset to defaults.");
                CONFIG.set_to_default();
            }
            else {
                log_critical!(&self.log, "Configuration could not be updated, terminating.");
    
                return Err( DaemonFailure::ConfigurationError );
            }
        }
    
        let results: [Option<RestartError>; 3] = [
            self.console.send_or_restart(ConsoleComm::ReloadConfiguration, true).await.err(),
            self.metric.send_or_restart(SimpleComm::ReloadConfiguration, true).await.err(),
            self.client.send_or_restart(SimpleComm::ReloadConfiguration, true).await.err()
        ];

        let send_failure = results.iter().all(|x| {
            if let Some(e) = x.as_ref() {
                log_critical!(&self.log, "Unable to send out configuration reload message due to error: {}", e);
                false
            }
            else {
                true
            }
        });
        
        if send_failure {
            log_critical!(&self.log, "Due to config failures, the orch will now shut down.");
            Err( DaemonFailure::ConfigurationError )
        }
        else {
            log_info!(&self.log, "Configurations reloaded.");
            Ok(())
        }
    }

    /// Will spawn the needed timing and shutdown tasks, and will conduct polls & restart tasks as needed.
    pub async fn run(mut self) -> Result<(), DaemonFailure> {
        // This thread will do polls to determine if threads need to be spanwed.

        //This needs a timer thread. At a periodic time, this timer will signal this main thread to send out poll requests. It only works with the unit type. If it receives a message, it will immediatley exit.
        log_info!(&self.log, "Spawning timer & SIGTERM threads...");
        let mut timer = interval(Duration::from_secs(TASK_CHECK_TIMEOUT));

        //Get the signals to await later on, to listen to the OS.
        let mut signals = match Self::get_signals() {
            Ok(v) => v,
            Err(e) => {
                log_critical!(&self.log, "Unable to generate SIGTERM, SIGINT, or SIGHUP signal handlers (Error: '{e}'). Aborting.");
                return Err( DaemonFailure::SignalFailure );
            }
        };

        log_info!(&self.log, "Utility signals/tasks loaded.");

        let mut err: Option<DaemonFailure> = None;
        // Critical code
        loop {
            select! {
                _ = timer.tick() => {
                    if !self.poll().await {
                        err = Some(DaemonFailure::UnexepctedError);
                        break;
                    }
                },
                _ = signals.term.recv() => {
                    log_info!(&self.log, "SIGTERM message from OS received, shutting down threads.");
                    break;
                },
                _ = signals.int.recv() => {
                    log_info!(&self.log, "SIGINT message from OS received, shutting down threads.");
                    break;
                }
                _ = signals.hup.recv() => {
                    log_info!(&self.log, "SIGHUP message received from OS, reloading configuration.");
                    if let Err(e) = self.reload_configuration().await {
                        err = Some(e);
                        break;
                    }
                    log_info!(&self.log, "Configuration reloaded.");
                }
                m = self.console.force_recv() => {
                    if let Some(m) = m {
                        match m {
                            ConsoleComm::ReloadConfiguration => {
                                log_info!(&self.log, "By request of console thread, the configuration is being reloaded...");
                                if let Err(e) = self.reload_configuration().await {
                                    err = Some(e);
                                    break;
                                }
                                log_info!(&self.log, "Configuration reloaded.");
                            },
                            ConsoleComm::SystemShutdown => {
                                log_info!(&self.log, "By request of console thread, the system is to shutdown...");
                                log_info!(&self.log, "Shutting down threads...");
                                break;
                            },
                            v => log_warning!(&self.log, "Got innapropriate request from console thread: '{v}'. Ignoring.")
                        }
                    }
                    else {
                        log_info!(&self.log, "Console thread unexpectedly closed. Attempting to restart...");
                        if self.console.restart(true).await.is_err() {
                            log_critical!(&self.log, "Unable to restart console thread. Shutting down tasks...");
                            err = Some(DaemonFailure::UnexepctedError);
                            break;
                        }
                        log_info!(&self.log, "onsole thread restarted.");
                    }
                }
            }
        }

        // Shutting down threads
        self.shutdown().await;

        if let Some(err) = err {
            Err(err)
        }
        else {
            Ok(()) 
        }
    }

    fn get_shutdown_msg(x: Result<WorkerTaskResult, JoinError>) -> String {
        match x {
            Ok(v) => v.to_string(),
            Err(e) => format!("join error: '{e}'"),
        }
    }

    pub async fn shutdown(self) {
        let shutdowns = [
            Self::get_shutdown_msg(self.client.join().await),
            Self::get_shutdown_msg(self.console.join().await),
            Self::get_shutdown_msg(self.metric.join().await)
        ];

        log_info!(
            &self.log, 
            "Client task shutdown with response '{}'",
            shutdowns[0]
        );
        log_info!(
            &self.log,
            "Console task shutdown with response '{}'",
            shutdowns[1]
        );
        log_info!(
            &self.log,
            "Metric task shutdown with response '{}'",
            shutdowns[2]
        );
        log_info!(&self.log, "Tasks shut down.");
    }
}
