use tokio::{
    sync::mpsc::{channel, Receiver, Sender, error::SendError}, 
    task::{
        JoinError, 
        JoinHandle
    }
};

use crate::message::{KillMessage, PollableMessage};

/// The buffer size for a default channel buffer. 
pub const TASKS_DEFAULT_BUFFER: usize = 10;

pub trait TaskBasis<T, Args> where T: Send + 'static {
    /// Spanws a task using `tokio::spawn`, and establishes communication between the tasks and this thread.
    fn start<F, Fut>(func: F) -> Self
        where Self: Sized,
        F: FnOnce(Args) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static;

    /// Resets the internal state of the object if the task is to be deleted.
    /// Fails if the task is still running.
    fn restart<F, Fut>(&mut self, func: F) -> bool 
        where Self: Sized,
        F: FnOnce(Args) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static {
        if self.is_running() {
            return false;
        }

        *self = Self::start(func);
        true
    } 

    /// Attempts to poll. If the poll fails, it attempts to restart. It returns the result of the restart, or `true` if the poll is successful. 
    async fn poll_and_restart<F, Fut>(&mut self, func: F) -> bool 
        where Self: Sized,
        F: FnOnce(Args) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        T: PollableMessage {
        if !self.poll().await {
            self.restart(func)
        }
        else {
            true
        }
    }

    fn join_handle(&self) -> &JoinHandle<()>;
    fn join_handle_owned(self) -> JoinHandle<()>;
    fn sender(&self) -> &Sender<T>;

    /// Determines if the holding task is still running.
    fn is_running(&self) -> bool {
        !self.join_handle().is_finished()
    }
    /// Waits for the inner task to finish completing.
    async fn join(self) -> Result<(), JoinError> where Self: Sized {
        self.join_handle_owned().await
    }
    /// If the task is currently running, it will send the 'T::kill()` value. After that, if the send was ok, it will join the handle. Note that errors are not considered nor recorded.
    async fn shutdown(self) where T: KillMessage + Send + 'static, Self: Sized{
        self.shutdown_explicit(T::kill()).await
    }
    /// If the task is currently running, it will send the `signal` value. After that, if the send was ok, it will join the handle. Note that errors are not considered nor recorded.
    async fn shutdown_explicit(self, signal: T) where Self: Sized {
        if self.is_running() {
            if self.send(signal).await.is_ok() {
                let _ = self.join().await;
            }
        }
    }

    /// Sends a message to the task.
    async fn send(&self, value: T) -> Result<(), SendError<T>> {
        self.sender().send(value).await
    }

    /// Sends a message to the inner task, if it is running, using the `T::poll()` value. If there is no error, it will return true. If the task is completed, or there is an sending error, it returns false.
    async fn poll(&mut self) -> bool where T: PollableMessage {
        if self.join_handle().is_finished() {
            return false;
        }
        else {
            self.send(T::poll()).await.is_ok()
        }
    }
}

/// A combination of required tools for accessing and communicating with tasks.
pub struct DuplexTask<T> where T: Send + 'static {
    join: JoinHandle<()>,
    sender: Sender<T>,
    receiver: Receiver<T>
}
impl<T> TaskBasis<T, (Sender<T>, Receiver<T>)> for DuplexTask<T> where T: Send + 'static  {
    fn start<F, Fut>(func: F) -> Self
            where Self: Sized,
            F: FnOnce((Sender<T>, Receiver<T>)) -> Fut + Send + 'static,
            Fut: Future<Output = ()> + Send + 'static {
        
        let (my_sender, their_recv) = channel::<T>(TASKS_DEFAULT_BUFFER);
        let (their_sender, my_recv) = channel::<T>(TASKS_DEFAULT_BUFFER);

        let handle = tokio::spawn(async move {
            (func)((their_sender, their_recv)).await
        });

        Self {
            join: handle,
            sender: my_sender,
            receiver: my_recv
        }
    }
    
    fn join_handle(&self) -> &JoinHandle<()> {
        &self.join
    }
    fn join_handle_owned(self) -> JoinHandle<()> {
        self.join
    }
    fn sender(&self) -> &Sender<T> {
        &self.sender
    }
}
impl<T> DuplexTask<T> where T: Send + 'static {
    /// Generates a `TaskHandle` from predetermined channels and join handle.
    pub fn new(join: JoinHandle<()>, sender: Sender<T>, receiver: Receiver<T>) -> Self {
        Self {
            join,
            sender,
            receiver
        }
    }

    pub async fn recv(&mut self) -> Option<T> {
        self.receiver.recv().await
    }
}

/// A combination of required tools for accessing and communicating with tasks.
pub struct SimplexTask<T> where T: Send + 'static {
    join: JoinHandle<()>,
    sender: Sender<T>,
}
impl<T> TaskBasis<T, Receiver<T>> for SimplexTask<T> where T: Send + 'static {
    fn start<F, Fut>(func: F) -> Self
            where Self: Sized,
            F: FnOnce(Receiver<T>) -> Fut + Send + 'static,
            Fut: Future<Output = ()> + Send + 'static {
        
        let (my_sender, their_recv) = channel::<T>(TASKS_DEFAULT_BUFFER);

        let handle = tokio::spawn(async move {
            (func)(their_recv).await
        });

        Self {
            join: handle,
            sender: my_sender
        }
    }
    
    fn join_handle(&self) -> &JoinHandle<()> {
        &self.join
    }
    fn join_handle_owned(self) -> JoinHandle<()> {
        self.join
    }
    fn sender(&self) -> &Sender<T> {
        &self.sender
    }
}
impl<T> SimplexTask<T> where T: Send + 'static {
    /// Generates a `TaskHandle` from predetermined channels and join handle.
    pub fn new(join: JoinHandle<()>, sender: Sender<T>) -> Self {
        Self {
            join,
            sender
        }
    }
}
