
use std::{process::ExitCode, task::Poll};
use tokio::{
    select, 
    signal::unix::{
        signal, 
        SignalKind
    }, 
    sync::mpsc, 
    task::{
        JoinError, 
        JoinHandle
    }
};
use std::time::Duration;

use crate::{
    log_warning,
    metric::mgnt::metrics_entry, 
    connect::{
        client::mgnt::client_entry, 
        console::mgnt::console_entry
    }
};

pub trait KillMessage : Send + Sized{
    fn kill() -> Self;
}
pub trait PollableMessage : Send + Sized {
    fn poll() -> Self;
}
/// A representation of communication between the `Orchestrator` and the client worker tasks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimpleComm {
    /// A request to see if the thread in question is working fine.
    Poll,
    /// A command to tell that task to stop executing.
    Kill,
    /// A message from the orchestrator to reload configuration. 
    ReloadConfiguration,
}
impl KillMessage for SimpleComm {
    fn kill() -> Self {
        Self::Kill
    }
}
impl PollableMessage for SimpleComm {
    fn poll() -> Self {
        Self::Poll
    }
}

/// A representation of communication between the `Orchestrator` and the console worker tasks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleComm {
    /// A request to see if the thread in question is working fine. 
    Poll,
    /// A command to tell that task to stop executing. 
    Kill,

    /// A message to the ochestrator to shutdown all tasks. 
    SystemShutdown,
    //// A message to the ochestrator to tell other tasks to reload configuration.
    ReloadConfiguration
}
impl KillMessage for ConsoleComm {
    fn kill() -> Self {
        Self::Kill
    }
}
impl PollableMessage for ConsoleComm {
    fn poll() -> Self {
        Self::Poll
    }
}

/// The amount of time between each task "poll". 
pub const TASK_CHECK_TIMEOUT: u64 = 40;
/// The buffer size for a default channel buffer. 
pub const TASKS_DEFAULT_BUFFER: usize = 10;

/// A combination of required tools for accessing and communicating with tasks.
pub struct TaskHandle<T> where T: Send + 'static {
    join: JoinHandle<()>,
    sender: mpsc::Sender<T>,
    receiver: mpsc::Receiver<T>
}
impl<T> TaskHandle<T> where T: Send + 'static {
    /// Generates a `TaskHandle` from predetermined channels and join handle.
    pub fn new(join: JoinHandle<()>, sender: mpsc::Sender<T>, receiver: mpsc::Receiver<T>) -> Self {
        Self {
            join,
            sender,
            receiver
        }
    }

    /// Spanws a task using `tokio::spawn`, and creates duplex channel communication. Lastly, bundles the required information together, and returns a `TaskHandle<T>` for later use.
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

    /// Resets the internal state of the object if the task is to be deleted.
    /// Fails if the task is still running.
    pub fn restart<F, Fut>(&mut self, func: F) -> bool 
        where F: FnOnce(mpsc::Sender<T>, mpsc::Receiver<T>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static {
        if self.is_running() {
            return false;
        }

        *self = Self::start(func);
        true
    } 

    /// Determines if the holding task is still running.
    pub fn is_running(&self) -> bool {
        !self.join.is_finished()
    }
    /// Waits for the inner task to finish completing.
    pub async fn join(self) -> Result<(), JoinError> {
        self.join.await
    }
    /// If the task is currently running, it will send the 'T::kill()` value. After that, if the send was ok, it will join the handle. Note that errors are not considered nor recorded.
    pub async fn shutdown(self) where T: KillMessage + Send + 'static {
        self.shutdown_explicit(T::kill()).await
    }
    /// If the task is currently running, it will send the `signal` value. After that, if the send was ok, it will join the handle. Note that errors are not considered nor recorded.
    pub async fn shutdown_explicit(self, signal: T) {
        if self.is_running() {
            if self.send(signal).await.is_ok() {
                let _ = self.join().await;
            }
        }
    }

    /// Sends a message to the task.
    pub async fn send(&self, value: T) -> Result<(), mpsc::error::SendError<T>> {
        self.sender.send(value).await
    }
    /// Receives a message from the task.
    pub async fn recv(&mut self) -> Option<T> {
        self.receiver.recv().await
    }

    /// Sends a message to the inner task, if it is running, using the `T::poll()` value. If there is no error, it will return true. If the task is completed, or there is an sending error, it returns false.
    pub async fn poll(&mut self) -> bool where T: PollableMessage {
        if self.join.is_finished() {
            return false;
        }
        else {
            self.send(T::poll()).await.is_ok()
        }
    }
}

pub struct Orchestrator {
    client_thread: TaskHandle<SimpleComm>,
    metric_thread: TaskHandle<SimpleComm>,
    console_thread: TaskHandle<ConsoleComm>
}
impl Orchestrator {
    pub fn initialize() -> Self{
        let client_thread = TaskHandle::start(client_entry);
        let console_thread = TaskHandle::start(console_entry);
        let metric_thread = TaskHandle::start(metrics_entry);

        Self {
            client_thread,
            console_thread,
            metric_thread
        }
    }

    /// Spawns a timer thread, used to establish some sort of needed task over time.
    pub fn spawn_timer() -> TaskHandle<()> {
        let function = async |sender: mpsc::Sender<()>, mut receiver: mpsc::Receiver<()>| -> () {
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

        TaskHandle::<()>::start(function)
    }

    /// Will spawn the needed timing and shutdown tasks, and will conduct polls & restart tasks as needed.
    pub async fn run(mut self) -> Result<(), ExitCode> {
        // This thread will do polls to determine if threads need to be spanwed. 

        //This needs a timer thread. At a periodic time, this timer will signal this main thread to send out poll requests. It only works with the unit type. If it receives a message, it will immediatley exit. 
        let mut timer_handle = Self::spawn_timer();

        // To handle SIGTERM, since this is a daemon, a separate task needs to be made 
        let mut term_signal = match signal(SignalKind::terminate()) {
            Ok(v) => v,
            Err(e) => panic!("Unable to generate sigterm signal handler '{e}'")
        };

        // Critical code
        loop {
            select! {
                p = timer_handle.recv() => {
                    match p {
                        Some(_) => {
                            if !self.client_thread.poll().await {
                                self.client_thread.restart(client_entry);
                            }

                            if !self.metric_thread.poll().await {
                                self.metric_thread.restart(metrics_entry);
                            }

                            if !self.console_thread.poll().await {
                                self.console_thread.restart(console_entry);
                            }
                        },
                        None => break
                    }
                },
                _ = term_signal.recv() => {
                    
                }, 
                m = self.client_thread.recv() => {

                },
                m = self.console_thread.recv() => {

                },
                m = self.metric_thread.recv() => {

                }
            }
        }

        // Shutting down threads
        timer_handle.shutdown_explicit(()).await;
        self.shutdown().await;

        Ok(())
    }

    pub async fn shutdown(self) {
        self.client_thread.shutdown().await;
        self.console_thread.shutdown().await;
        self.metric_thread.shutdown().await;
    }
}