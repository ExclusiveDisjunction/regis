use tokio::{
    sync::mpsc::{channel, Receiver, Sender, error::SendError}, 
    task::{
        JoinError, 
        JoinHandle
    }
};

use crate::message::{KillMessage, PollableMessage};

/// An abstraction over communication channel(s) and a join handle. It handles the spawning and resulting of different async tasks.
pub trait TaskBasis {
    type Msg: Send + 'static;
    type Output: Send + 'static;
    type Arg: Send + 'static;

    /// Retreives the join handle from the task as a reference.
    fn join_handle(&self) -> &JoinHandle<Self::Output>;
    /// Consumes this object and returns the join handle from within.
    fn join_handle_owned(self) -> JoinHandle<Self::Output>;
    /// Gets the sender used by this task.
    fn sender(&self) -> &Sender<Self::Msg>;

    /// Determines if the holding task is still running.
    fn is_running(&self) -> bool {
        !self.join_handle().is_finished()
    }
}
/// Represents a task that can also receive messgaes from the inner task.
pub trait RecvTaskBasis: TaskBasis {
    /// Gets the receiver as a reference.
    fn receiver(&self) -> &Receiver<Self::Msg>;
    /// Getst he receiver as a mutable reference
    fn receiver_mut(&mut self) -> &mut Receiver<Self::Msg>;
}
pub trait StartableTask: TaskBasis where Self: Sized {
        /// Spanws a task using `tokio::spawn`, and establishes communication between the tasks and this thread.
        fn start<F, Fut>(func: F, buffer_size: usize) -> Self where 
            Self: Sized,
            F: FnOnce(Self::Arg) -> Fut + Send + 'static,
            Fut: Future<Output = Self::Output> + Send + 'static;

        /// Resets the internal state of the object if the task is to be deleted.
        /// Fails if the task is still running.
        fn restart<F, Fut>(&mut self, func: F, buffer_size: usize) -> bool where 
            Self: Sized,
            F: FnOnce(Self::Arg) -> Fut + Send + 'static,
            Fut: Future<Output = Self::Output> + Send + 'static {
                if self.is_running() {
                    return false;
                }

                *self = Self::start(func, buffer_size);
                true
        } 
}

pub struct RestartableTask<Task> 
    where Task: StartableTask  {
        task: Task,
        restart_count: u8,
        max_restart: u8,
}
impl<Task> TaskBasis for RestartableTask<Task>   
    where Task: StartableTask {
        type Arg = Task::Arg;
        type Msg = Task::Msg;
        type Output = Task::Output;

        fn join_handle(&self) -> &JoinHandle<Self::Output> {
            self.task.join_handle()
        }
        fn join_handle_owned(self) -> JoinHandle<Self::Output> {
            self.task.join_handle_owned()
        }
        fn sender(&self) -> &Sender<Self::Msg> {
            self.task.sender()
        }
}
impl<Task> RecvTaskBasis for RestartableTask<Task>
    where Task: StartableTask + RecvTaskBasis {
        fn receiver(&self) -> &Receiver<Self::Msg> {
            self.task.receiver()
        }
        fn receiver_mut(&mut self) -> &mut Receiver<Self::Msg> {
            self.task.receiver_mut()
        }
    }
impl<Task> RestartableTask<Task>   
    where Task: StartableTask {
        /// Establishes a restartable task with a specific `max_restart` value.
        /// # Panics
        /// If `max_restart` or `buffer_size` is `0`, this will panic.
        pub fn start<F, Fut>(func: F, buffer_size: usize, max_restart: u8) -> Self where
            F: FnOnce(Task::Arg) -> Fut + Send + 'static,
            Fut: Future<Output = Task::Output> + Send + 'static {
                if max_restart == 0 || buffer_size == 0 {
                    panic!("The max restart cannot be zero.");
                }

                let task = Task::start(func, buffer_size);

                Self {
                    task,
                    restart_count: 0,
                    max_restart
                }
        }

        pub fn restart<F, Fut>(&mut self, func: F, buffer_size: usize) -> bool where
            F: FnOnce(Task::Arg) -> Fut + Send + 'static,
            Fut: Future<Output = Task::Output> + Send + 'static {
                if buffer_size == usize::MAX || self.restart_count + 1 == self.max_restart {
                    return false;
                }

                self.restart_count += 1;
                self.task.restart(func, buffer_size);

                true
        }

        pub async fn poll_and_restart<F, Fut>(&mut self, func: F, buffer_size: usize) -> bool where
            F: FnOnce(Task::Arg) -> Fut + Send + 'static,
            Fut: Future<Output = Task::Output> + Send + 'static,  
             Task::Msg: PollableMessage {
                poll(&mut self.task).await || self.restart(func, buffer_size)
        }
}

/// Waits for the inner task to finish completing.
pub async fn join<Task>(handle: Task) -> Result<Task::Output, JoinError> where Task: TaskBasis {
    handle.join_handle_owned().await
}
/// If the task is currently running, it will send the 'T::kill()` value. After that, if the send was ok, it will join the handle. Note that errors are not considered nor recorded.
pub async fn shutdown<Task>(handle: Task) -> Result<Option<Task::Output>, JoinError> where Task: TaskBasis, Task::Msg: KillMessage {
    shutdown_explicit(handle, Task::Msg::kill()).await
}
/// If the task is currently running, it will send the `signal` value. After that, if the send was ok, it will join the handle. Note that errors are not considered nor recorded.
pub async fn shutdown_explicit<Task>(handle: Task,  signal: Task::Msg) -> Result<Option<Task::Output>, JoinError> where Task: TaskBasis{
    if handle.is_running() && send(&handle, signal).await.is_ok() {
        join(handle).await.map(Some)
    }
    else {
        Ok(None)
    }

    
}

/// Sends a message to the task.
pub async fn send<Task>(handle: &Task, value: Task::Msg) -> Result<(), SendError<Task::Msg>> where Task: TaskBasis {
    handle.sender().send(value).await
}
/// Receives a message from the task.
pub async fn recv<Task>(handle: &mut Task) -> Option<Task::Msg> where Task: RecvTaskBasis {
    handle.receiver_mut().recv().await
}

/// Sends a message to the inner task, if it is running, using the `T::poll()` value. If there is no error, it will return true. If the task is completed, or there is an sending error, it returns false.
pub async fn poll<Task>(handle: &mut Task) -> bool where Task: TaskBasis, Task::Msg: PollableMessage {
    if handle.join_handle().is_finished() {
        false
    }
    else {
        send(handle, Task::Msg::poll()).await.is_ok()
    }
}

/// A combination of required tools for accessing and communicating with tasks.
pub struct DuplexTask<T, O> where T: Send + 'static, O: Sized {
    join: JoinHandle<O>,
    sender: Sender<T>,
    receiver: Receiver<T>
}
impl<T, O> TaskBasis for DuplexTask<T, O> where T: Send + 'static, O: Send + 'static {
    type Arg = (Sender<T>, Receiver<T>);
    type Msg = T;
    type Output = O;
    fn join_handle(&self) -> &JoinHandle<O> {
        &self.join
    }
    fn join_handle_owned(self) -> JoinHandle<O> {
        self.join
    }
    fn sender(&self) -> &Sender<T> {
        &self.sender
    }
}
impl<T, O> StartableTask for DuplexTask<T, O> where T: Send + 'static, O: Send + 'static {
    fn start<F, Fut>(func: F, buffer_size: usize) -> Self
            where Self: Sized,
            F: FnOnce(Self::Arg) -> Fut + Send + 'static,
            Fut: Future<Output = Self::Output> + Send + 'static {
        
        let (my_sender, their_recv) = channel::<Self::Msg>(buffer_size);
        let (their_sender, my_recv) = channel::<Self::Msg>(buffer_size);

        let handle = tokio::spawn(async move {
            (func)((their_sender, their_recv)).await
        });

        Self {
            join: handle,
            sender: my_sender,
            receiver: my_recv
        }
    }
}
impl<T, O> RecvTaskBasis for DuplexTask<T, O> where T: Send + 'static, O: Send + 'static {
    fn receiver(&self) -> &Receiver<T> {
        &self.receiver
    }
    fn receiver_mut(&mut self) -> &mut Receiver<T> {
        &mut self.receiver
    }
}
impl<T, O> DuplexTask<T, O> where T: Send + 'static, O: Send + 'static  {
    /// Generates a `TaskHandle` from predetermined channels and join handle.
    pub fn new(join: JoinHandle<O>, sender: Sender<T>, receiver: Receiver<T>) -> Self {
        Self {
            join,
            sender,
            receiver
        }
    }
}

/// A combination of required tools for accessing and communicating with tasks.
pub struct SimplexTask<T, O> where T: Send + 'static, O: Send + 'static {
    join: JoinHandle<O>,
    sender: Sender<T>,
}
impl<T, O> TaskBasis for SimplexTask<T, O> where T: Send + 'static, O: Send + 'static {
    type Arg = Receiver<T>;
    type Msg = T;
    type Output = O;

    fn join_handle(&self) -> &JoinHandle<O> {
        &self.join
    }
    fn join_handle_owned(self) -> JoinHandle<O> {
        self.join
    }
    fn sender(&self) -> &Sender<T> {
        &self.sender
    }
}
impl<T, O> StartableTask for SimplexTask<T, O> where T: Send + 'static, O: Send + 'static {
    fn start<F, Fut>(func: F, buffer_size: usize) -> Self
            where Self: Sized,
            F: FnOnce(Receiver<T>) -> Fut + Send + 'static,
            Fut: Future<Output = O> + Send + 'static {
        
        let (my_sender, their_recv) = channel::<T>(buffer_size);

        let handle = tokio::spawn(async move {
            (func)(their_recv).await
        });

        Self {
            join: handle,
            sender: my_sender
        }
    }
}
impl<T, O> SimplexTask<T, O> where T: Send + 'static, O: Send + 'static{
    /// Generates a `TaskHandle` from predetermined channels and join handle.
    pub fn new(join: JoinHandle<O>, sender: Sender<T>) -> Self {
        Self {
            join,
            sender
        }
    }
}