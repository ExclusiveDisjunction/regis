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
    lock::OptionRwProvider, 
    log_critical, 
    log_debug, 
    log_error, 
    log_info, 
    log_warning, 
    task_util::{
        recv, 
        send, 
        shutdown, 
        DuplexTask, 
        RestartStatusBase, 
        RestartableTask, 
        SimplexTask, 
        StartableTask
    }
};

/// The amount of time between each task "poll".
pub const TASK_CHECK_TIMEOUT: u64 = 30;
/// The buffer size for a default channel buffer.
pub const TASKS_DEFAULT_BUFFER: usize = 10;

async fn try_send_or_restart<T>(handle: &mut RestartableTask<T>, value: T::Msg) -> bool 
where T: StartableTask,
T::Output: RestartStatusBase{
    if let Err(e) = send(handle, value).await {
        let result = handle.restart().await;
        log_info!("(Orch) When sending message to task, the task was dead (Error: '{e}'). Could it be restarted? '{}'", result.is_ok());

        result.is_err()
    }
    else {
        true
    }
}

struct SignalBundle {
    term: Signal,
    int: Signal,
    hup: Signal
}

pub struct Orchestrator {
    client_thread: RestartableTask<SimplexTask<SimpleComm, WorkerTaskResult>>,
    metric_thread: RestartableTask<SimplexTask<SimpleComm, WorkerTaskResult>>,
    console_thread: RestartableTask<DuplexTask<ConsoleComm, WorkerTaskResult>>,
    options: Options
}
impl Orchestrator {
    pub fn initialize(options: setup::Options) -> Self {
        let client_thread = RestartableTask::start(client_entry, TASKS_DEFAULT_BUFFER, 5);
        let console_thread = RestartableTask::start(console_entry, TASKS_DEFAULT_BUFFER, 5);
        let metric_thread = RestartableTask::start(metrics_entry, TASKS_DEFAULT_BUFFER, 5);

        Self {
            client_thread,
            console_thread,
            metric_thread,
            options
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
        log_debug!("(Orch) Timer tick activated");
        log_info!("(Orch) Beginning polls...");
        let mut result: bool = true;

        result &= self.client_thread.poll_and_restart().await.log_event("client");
        result &= self.metric_thread.poll_and_restart().await.log_event("metrics");
        result &= self.console_thread.poll_and_restart().await.log_event("console");

        if !result {
            log_info!("(Orch) Polls complete, failure.");
            log_critical!("Unable to restart one or more worker threads, shutting down all threads.");
            return false;
        }

        log_info!("(Orch) Polls complete, success.");
        true
    }

    async fn reload_configuration(&mut self) -> Result<(), DaemonFailure> {
        if let Err(e) = CONFIG.open(DAEMON_CONFIG_PATH) {
            log_error!("(Orch) Unable to reload configuration, due to '{:?}'.", e);
            if self.options.override_config {
                log_info!("By request, the configuration will be reset to defaults.");
                CONFIG.set_to_default();
            }
            else {
                log_critical!("(Orch) Configuration could not be updated, terminating.");
    
                return Err( DaemonFailure::ConfigurationError );
            }
        }
    
        let mut send_failure: bool = false;
        send_failure &= try_send_or_restart(&mut self.console_thread, ConsoleComm::ReloadConfiguration).await;
        send_failure &= try_send_or_restart(&mut self.metric_thread, SimpleComm::ReloadConfiguration).await;
        send_failure &= try_send_or_restart(&mut self.client_thread, SimpleComm::ReloadConfiguration).await;
        
        if send_failure {
            log_critical!("(Orch) Unable to send out configuration reloading messages, as some threads were dead and could not be restarted.");
            Err( DaemonFailure::ConfigurationError )
        }
        else {
            log_info!("(Orch) Configurations reloaded.");
            Ok(())
        }
    }

    /// Will spawn the needed timing and shutdown tasks, and will conduct polls & restart tasks as needed.
    pub async fn run(mut self) -> Result<(), DaemonFailure> {
        // This thread will do polls to determine if threads need to be spanwed.

        //This needs a timer thread. At a periodic time, this timer will signal this main thread to send out poll requests. It only works with the unit type. If it receives a message, it will immediatley exit.
        log_info!("(Orch) Spawning timer & SIGTERM threads...");
        let mut timer = interval(Duration::from_secs(TASK_CHECK_TIMEOUT));

        //Get the signals to await later on, to listen to the OS.
        let mut signals = match Self::get_signals() {
            Ok(v) => v,
            Err(e) => {
                log_critical!("(Orch) Unable to generate SIGTERM, SIGINT, or SIGHUP signal handlers (Error: '{e}'). Aborting.");
                return Err( DaemonFailure::SignalFailure );
            }
        };

        log_info!("(Orch) Utility signals/tasks loaded.");

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
                    log_info!("(Orch) SIGTERM message from OS received, shutting down threads.");
                    break;
                },
                _ = signals.int.recv() => {
                    log_info!("(Orch) SIGINT message from OS received, shutting down threads.");
                    break;
                }
                _ = signals.hup.recv() => {
                    log_info!("(Orch) SIGHUP message received from OS, reloading configuration.");
                    if let Err(e) = self.reload_configuration().await {
                        err = Some(e);
                        break;
                    }
                    log_info!("(Orch) Configuration reloaded.");
                }
                m = recv(&mut self.console_thread) => {
                    if let Some(m) = m {
                        match m {
                            ConsoleComm::ReloadConfiguration => {
                                log_info!("(Orch) By request of console thread, the configuration is being reloaded...");
                                if let Err(e) = self.reload_configuration().await {
                                    err = Some(e);
                                    break;
                                }
                                log_info!("(Orch) Configuration reloaded.");
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
                        if self.console_thread.restart().await.is_err() {
                            log_critical!("Unable to restart console thread. Shutting down tasks...");
                            err = Some(DaemonFailure::UnexepctedError);
                            break;
                        }
                        log_info!("(Orch) Console thread restarted.");
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

    fn get_shutdown_msg(x: Result<Option<WorkerTaskResult>, JoinError>) -> String {
        match x {
            Ok(v) => match v {
                Some(v) => v.to_string(),
                None => "Already Joined".to_string(),
            },
            Err(e) => format!("join error: '{e}'"),
        }
    }

    pub async fn shutdown(self) {
        let shutdowns = vec![
            shutdown(self.client_thread).await,
            shutdown(self.console_thread).await,
            shutdown(self.metric_thread).await
        ];
        let mut iter = shutdowns.into_iter()
            .map(Self::get_shutdown_msg);

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
