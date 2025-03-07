
use std::process::ExitCode;
use tokio::{
    sync::mpsc,
    sync::oneshot,
    select,
    signal::unix::{signal, SignalKind},
    task::JoinHandle
};
//use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use crate::log_warning;
use crate::metric::mgnt::metrics_entry;
use crate::connect::{
    client::mgnt::client_entry, 
    console::mgnt::console_entry
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientComm {
    /// A request to see if the thread in question is working fine.
    Poll,
    /// A command to tell that task to stop executing.
    Kill,
    /// A message from the orchestrator to reload configuration. 
    ReloadConfiguration,

    /// A response to a poll that indicates that the task in question has connections.
    Busy,
    /// A response to a poll that indicates that the task in question has no connections, but is active. 
    Inactive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleComm {
    /// A request to see if the thread in question is working fine. 
    Poll,
    /// A command to tell that task to stop executing. 
    Kill,

    /// A message to the ochestrator to shutdown all tasks. 
    SystemShutdown,
    //// A message to the ochestrator to tell other tasks to reload configuration.
    ReloadConfiguration,
    
    /// A response to a poll that indicates that the task in question has connections.
    Busy,
    /// A response to a poll that indicates that the task in question has no connections, but is active. 
    Inactive
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricComm {
    /// A request to see if the thread in question is working fine.
    /// Note that the metric thread need not respond to this message. The worker must simply empty the message queue. 
    Poll,
    /// A command to tell that task to stop executing.
    Kill,

    /// A command to tell the task to reload its configuration settings.
    ReloadConfiguration
}

/// The amount of time between each task "poll". 
pub const TASK_CHECK_TIMEOUT: u64 = 40;
pub const TASKS_DEFAULT_BUFFER: usize = 10;

/// A combination of required tools for accessing and communicating with tasks.
pub struct TaskHandle<T> where T: Copy + Clone + Send + 'static {
    join: JoinHandle<()>,
    sender: mpsc::Sender<T>,
    receiver: mpsc::Receiver<T>
}
impl<T> TaskHandle<T> where T: Copy + Clone + Send + 'static {
    pub fn new(join: JoinHandle<()>, sender: mpsc::Sender<T>, receiver: mpsc::Receiver<T>) -> Self {
        
    }

    pub fn start<F, Fut>(func: F) -> Self 
    where F: FnOnce(mpsc::Sender<T>, mpsc::Receiver<T>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static {
        let (my_sender, their_recv) = mpsc::channel::<T>(TASKS_DEFAULT_BUFFER);
        let (their_sender, my_recv) = mpsc::channel::<T>(TASKS_DEFAULT_BUFFER);

        let handle = tokio::spawn(async move {
            (func)(their_sender, their_recv).await
        });

        Self {
            join: handle,
            sender: my_sender,
            receiver: my_recv
        }
    }
}


pub struct Orchestrator {
    recv: mpsc::Receiver<Comm>,
    sender: mpsc::Sender<Comm>,
    client_thread: (JoinHandle<()>, mpsc::Sender<ClientComm>, mpsc::Receiver<ClientComm>),
    metric_thread: RunningThread,
    console_thread: RunningThread
}
impl Orchestrator {
    pub fn initialize() -> Self{
        let (sender, receiver) = mpsc::channel::<Comm>(30);

        let client_thread = Self::blank_start(client_entry, sender.clone());
        let console_thread= Self::blank_start(console_entry, sender.clone());
        let metric_thread = Self::blank_start(metrics_entry, sender.clone());

        Self {
            recv: receiver,
            sender,
            client_thread,
            console_thread,
            metric_thread
        }
    }

    fn start<F, Fut>(loc: &mut RunningThread, func: F, sender: mpsc::Sender<Comm>) 
        where F: FnOnce(mpsc::Sender<Comm>, mpsc::Receiver<Comm>) -> Fut + Send + 'static, 
        Fut: Future<Output = ()> + Send + 'static {
        *loc = Self::blank_start(func, sender);
    }

    fn blank_start<F, Fut>(func: F, sender: mpsc::Sender<Comm>) -> RunningThread 
        where F: FnOnce(mpsc::Sender<Comm>, mpsc::Receiver<Comm>) -> Fut + Send + 'static, 
        Fut: Future<Output = ()> + Send + 'static {

            let (curr_sender, curr_recv) = mpsc::channel(30);

            let handle = tokio::spawn(async move {
                (func)(sender, curr_recv).await
            });

            (
                handle,
                curr_sender
            )
    }

    /// Spawns a timer thread, used to establish some sort of needed task over time.
    pub fn spawn_timer() -> (mpsc::Sender<()>, mpsc::Receiver<()>, JoinHandle<()>) {
        let (inner_ts, timer_receiver) = mpsc::channel::<()>(2);
        let (timer_sender, mut inner_tr) = mpsc::channel::<()>(2);
        let timer_handle = tokio::spawn(async move {
            loop {
                select! {
                    _ = tokio::time::sleep(Duration::from_secs(TASK_CHECK_TIMEOUT)) => {
                        if inner_ts.send(()).await.is_err() {
                            return;
                        }
                    },
                    _ = inner_tr.recv() => {
                        return; //Getting anything is considering a close, error or not.
                    }
                }
            }
        });

        (timer_sender, timer_receiver, timer_handle)
    }

    /// Will spawn the needed timing and shutdown tasks, and will conduct polls & restart tasks as needed.
    pub async fn run(&mut self) -> Result<(), ExitCode> {
        // This thread will do polls to determine if threads need to be spanwed. 

        //This needs a timer thread. At a periodic time, this timer will signal this main thread to send out poll requests. It only works with the unit type. If it receives a message, it will immediatley exit. 
        let (timer_sender, mut timer_receiver, timer_handle) = Self::spawn_timer();

        // To handle SIGTERM, since this is a daemon, a separate task needs to be made 
        let mut term_signal = match signal(SignalKind::terminate()) {
            Ok(v) => v,
            Err(e) => panic!("Unable to generate sigterm signal handler '{e}'")
        };

        // Critical code
        loop {
            select! {
                p = timer_receiver.recv() => {
                    match p {
                        Some(_) => {
                            
                        },
                        None => break
                    }
                },
                v = self.recv.recv() => {

                },
                _ = term_signal.recv() => {

                }
            }
        }

        // Shutting down timer thread
        if let Err(e) = timer_sender.send(()).await {
            log_warning!("When shutting down timer thread, got error '{e}'");
        }

        if let Err(e) = timer_handle.await {
            log_warning!("Timer thread could not be joined because of '{e}'")
        }

        Ok(())
    }

    pub async fn join(self) {
        //First we will send a quit message to each availible thread.
        // At this step, if a send fails because that thread paniced, we will ignore the error, but log it. 

        let working = vec![self.client_thread, self.metric_thread, self.console_thread];
        for (_, sender) in &working {
            if let Err(e) = sender.send(Comm::Req(Request::Kill)).await {
                log_warning!("Unable to send to worker thread (Request Kill), error message '{:?}'", e);
            }
        }

        //Now we attempt to close all threads.
        for (t, _) in working {
            if let Err(e) = t.await {
                log_warning!("Unable to join worker thread, error message '{:?}'", &e);
            }
        }
    }
}