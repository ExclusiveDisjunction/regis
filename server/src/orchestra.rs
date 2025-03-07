
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
pub enum Request {
    Poll, //Asks the thread its current status 
    Kill, //Tells the thread to stop executing and clean up resources 
    SystemShutdown, // A client to controller message to shutdown the whole server
    Panic(bool), // A message that indicates a panic. If the value contained is true, the controller should restart the task. 
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response {
    Busy, //The task has an active connection
    Inactive, //The task has no active work, but is listening for work.
    Ok, //Everything is ok
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comm {
    Req(Request),
    Resp(Response)
}
impl From<Request> for Comm {
    fn from(value: Request) -> Self {
        Self::Req(value)
    }
}
impl From<Response> for Comm {
    fn from(value: Response) -> Self {
        Self::Resp(value)
    }
}

type RunningThread = (tokio::task::JoinHandle<()>, mpsc::Sender<Comm>);

/// The amount of time between each task "poll". 
pub const TASK_CHECK_TIMEOUT: u64 = 40;

pub struct Orchestrator {
    recv: mpsc::Receiver<Comm>,
    sender: mpsc::Sender<Comm>,
    client_thread: RunningThread,
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

    /// Spwans a task that will listen for the SIGTERM event. 
    fn spawn_shutdown() -> (oneshot::Sender<()>, mpsc::Receiver<()>, JoinHandle<()>) {
        let mut term_signal = match signal(SignalKind::terminate()) {
            Ok(v) => v,
            Err(e) => panic!("Unable to generate sigterm signal handler '{e}'")
        };

        let (shutdown_s, shutdown_r) = mpsc::channel::<()>(1);
        let (signal_s, signal_r) = oneshot::channel::<()>();
        let signal_handle = tokio::spawn(async move {
            select! {
                _ = term_signal.recv() => {
                    let _ = shutdown_s.send(()).await;
                },
                _ = signal_r => {
                    return;
                }
            }
        });

        (signal_s, shutdown_r, signal_handle)
    }

    /// Will spawn the needed timing and shutdown tasks, and will conduct polls & restart tasks as needed.
    pub async fn run(&mut self) -> Result<(), ExitCode> {
        // This thread will do polls to determine if threads need to be spanwed. 
        
        // To handle SIGTERM, since this is a daemon, a separate task will be made. 
        let (shutdown_signal, mut shutdown_r, shutdown_handle) = Self::spawn_shutdown();

        //This needs a timer thread. At a periodic time, this timer will signal this main thread to send out poll requests. It only works with the unit type. If it receives a message, it will immediatley exit. 
        let (timer_sender, mut timer_receiver, timer_handle) = Self::spawn_timer();

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
                _ = shutdown_r.recv() => {

                }
            }
        }

        // Shutting down timer thread
        if let Err(e) = timer_sender.send(()).await {
            log_warning!("When shutting down timer thread, got error '{e}'");
        }
        let _ = shutdown_signal.send(()); //Tell the SIGTERM listener to stop listening 

        if let Err(e) = timer_handle.await {
            log_warning!("Timer thread could not be joined because of '{e}'")
        }
        if let Err(e) = shutdown_handle.await {
            log_warning!("Shutown thread could not be joined because of '{e}'");
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