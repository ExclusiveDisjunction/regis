
use std::process::ExitCode;
use std::thread::{self, JoinHandle};
use tokio::{
    spawn,
    sync::oneshot,
    sync::mpsc
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
pub enum ThreadRequest {
    Poll, //Asks the thread its current status 
    Kill, //Tells the thread to stop executing and clean up resources 
    Drop //Tells the thread to drop its current connection
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadResponse {
    Busy,
    Inactive,
    Ok,
}

pub enum Comm {
    Req(ThreadRequest),
    Resp(ThreadResponse)
}
impl From<ThreadRequest> for Comm {
    fn from(value: ThreadRequest) -> Self {
        Self::Req(value)
    }
}
impl From<ThreadResponse> for Comm {
    fn from(value: ThreadResponse) -> Self {
        Self::Resp(value)
    }
}

type RunningThread = (tokio::task::JoinHandle<()>, mpsc::Sender<Comm>);

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

    fn start<T>(loc: &mut RunningThread, func: T, sender: mpsc::Sender<Comm>) where T: AsyncFn(mpsc::Sender<Comm>, mpsc::Receiver<Comm>) -> () + Send + 'static {
        *loc = Self::blank_start(func, sender);
    }
    fn blank_start<T>(func: T, 
        sender: mpsc::Sender<Comm>) -> RunningThread
        where T: AsyncFn(mpsc::Sender<Comm>, mpsc::Receiver<Comm>) -> () 
            + Send + 'static {
        let (curr_sender, curr_recv) = mpsc::channel(30);



        (
            tokio::spawn(async move {
                (func)(sender, curr_recv).await
            }),
            curr_sender
        )
    }

    pub async fn run(&mut self) -> Result<(), ExitCode> {
        // This thread will do polls to determine if threads need to be spanwed. 
        
        //This needs a timer thread. The timer thread will send poll requests to this thread when it needs to do a poll. This thread will async await for a specifc message coming in. 

        let send = self.sender.clone();
        let (ts, mut tr) = oneshot::channel();
        let timer_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(15)).await;

                // Gracefully exit if requested to do so.
                if matches!(
                    tr.try_recv(),
                    Ok(ThreadRequest::Kill) | Err(oneshot::error::TryRecvError::Closed)
                ) {
                    return;
                }

                // If there is an error with sending, this thread will gracefully exit.
                if send.send(ThreadRequest::Poll.into()).await.is_err() {
                    return;
                }
            }
        });

        // Critical code

        // Shutting down timer thread
        if let Err(e) = ts.send(ThreadRequest::Kill) {
            log_warning!("When shutting down timer thread, got error '{:?}'", e);
        }
        if let Err(e) = timer_handle.await {
            log_warning!("Timer thread could not be joined because of '{:?}'", e)
        }

        Ok(())
    }

    pub async fn join(self) {
        //First we will send a quit message to each availible thread.
        // At this step, if a send fails because that thread paniced, we will ignore the error, but log it. 

        let working = vec![self.client_thread, self.metric_thread, self.console_thread];
        for (_, sender) in &working {
            if let Err(e) = sender.send(Comm::Req(ThreadRequest::Kill)) {
                log_warning!("Unable to send to worker thread (Request Kill), error message '{:?}'", e);
            }
        }

        //Now we attempt to close all threads.
        for (t, _) in working {
            if let Err(e) = t.join() {
                log_warning!("Unable to join worker thread, error message '{:?}'", &e);
            }
        }
    }
}