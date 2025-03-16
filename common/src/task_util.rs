use std::{fmt::Display, marker::PhantomData};

use tokio::{
    select, sync::mpsc::{channel, error::SendError, Receiver, Sender}, task::{
        JoinError,
        JoinHandle
    }
};

use crate::{log_debug, log_warning, log_error, log_critical};

/// A specific object that has a "kill" message value, such that if passed into a thread that listens to this message kind, it will stop executing.
pub trait KillMessage : Send + Sized{
    fn kill() -> Self;
}
/// A specific object that has a "poll"  message value, such that if passed into a thread that listens to this message kind, it will ignore it. 
pub trait PollableMessage : Send + Sized {
    fn poll() -> Self;
}

/// Representation of the success/failures of restarting a task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartStatus {
    Ok,
    WasDead,
    TriesExceeded,
    Argument
}
impl Display for RestartStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ok => "Ok",
                Self::WasDead => "was dead",
                Self::TriesExceeded => "restart tries exceeded",
                Self::Argument => "invalid argument passed"
            }
        )
    }
}
impl RestartStatus {
    /// Determines if the status is the ok variant. 
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }
    /// Determines if the status is an error variant.
    pub fn is_err(&self) -> bool {
        !matches!(self, Self::Ok)
    }
    /// Determines if the thread was restarted (warning value)
    pub fn was_restarted(&self) -> bool {
        matches!(self, Self::Ok | Self::WasDead)
    }

    /// Writes to the logger depending on the result of the poll. Note that 'Ok' values are only printed in debug mode.
    pub fn log_event(&self, name: &str) -> bool {
        match self {
            Self::Ok => (), //log_debug!("Poll of thread '{}' was ok", name),
            Self::WasDead => log_warning!("Poll of thread '{}' determined it was dead, but was successfully restarted.", name),
            Self::TriesExceeded => log_critical!("Poll of thread '{}' determined it was dead, but cannot be restarted.", name),
            Self::Argument => log_error!("Poll of thread '{}' determined that an argument passed into it was invalid, and could not be restarted.", name)
        }

        self.was_restarted()
    }
}

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

        /// Attempts to restart the current task.
        /// If the `buffer_size` is zero, it will return `RestartStatus::Argument`.
        /// Otherwise, it will restart the task, if it was dead. If it was dead, this will return `RestartStatus::WasDead`, otherwise, `RestartStatus::Ok`.
        fn restart<F, Fut>(&mut self, func: F, buffer_size: usize) -> RestartStatus where
            Self: Sized,
            F: FnOnce(Self::Arg) -> Fut + Send + 'static,
            Fut: Future<Output = Self::Output> + Send + 'static {
                if self.is_running() {
                    RestartStatus::Ok
                }
                else if buffer_size == 0 {
                    RestartStatus::Argument
                }
                else {
                    *self = Self::start(func, buffer_size);
                    RestartStatus::WasDead
                }
        }
}

/// A wrapper around a task that can be restarted. It requires that the task is a `StartableTask`. It will keep track of how many times the task will be restarted.
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

        /// Attempts to restart the current task.
        /// If the `buffer_size` is zero, it will return `RestartStatus::Argument`.
        /// If the amount of restarts is greater than or requal to the `max_restart`, it will return `RestartStatus::TriesExceeded`
        /// Otherwise, it will restart the task, if it was dead. If it was dead, this will return `RestartStatus::WasDead`, otherwise, `RestartStatus::Ok`.
        pub fn restart<F, Fut>(&mut self, func: F, buffer_size: usize) -> RestartStatus where
            F: FnOnce(Task::Arg) -> Fut + Send + 'static,
            Fut: Future<Output = Task::Output> + Send + 'static {
                if self.restart_count + 1 == self.max_restart {
                    RestartStatus::TriesExceeded
                }
                else {
                    self.restart_count += 1;
                    self.task.restart(func, buffer_size)
                }
        }

        /// Performs a poll on the task. If the poll fails, it will restart the task according to `self.restart`.
        pub async fn poll_and_restart<F, Fut>(&mut self, func: F, buffer_size: usize) -> RestartStatus where
            F: FnOnce(Task::Arg) -> Fut + Send + 'static,
            Fut: Future<Output = Task::Output> + Send + 'static,
             Task::Msg: PollableMessage {

                if !poll(&mut self.task).await {
                    self.restart(func, buffer_size)
                }
                else {
                    RestartStatus::Ok
                }
        }

        pub fn can_restart(&self) -> bool {
            self.restart_count >= self.max_restart
        }
        pub fn restarts_left(&self) -> u8 {
            if self.restart_count >= self.max_restart {
                0
            }
            else {
                self.max_restart - self.restart_count
            }
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
        let handle = handle.join_handle_owned();
        let abort_handle = handle.abort_handle();

        select! { 
            v = handle => {
                v.map(Some)
            },
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                log_warning!("(Shutdown) A task did not respond to shutdown signal, force aborting.");
                abort_handle.abort();
                Ok(None)
            }
        }
    }
    else {
        Ok(None)
    }
}
/// Calls `shutdown` on multiple tasks, returning the results of each joined handle.
pub async fn shutdown_tasks<T>(tasks: Vec<T>) -> Vec<Result<Option<T::Output>, JoinError>> where T: TaskBasis, T::Msg: KillMessage {
    let mut result = Vec::with_capacity(tasks.len());
    for task in tasks {
        result.push(
            shutdown(task).await
        )
    }

    result
}
/// Calls `shutdown_explicit` on multiple tasks, returning the results of each joined handle.
pub async fn shutdown_tasks_explicit<T>(tasks: Vec<T>, signal: T::Msg) -> Vec<Result<Option<T::Output>, JoinError>> where T: TaskBasis, T::Msg: Clone{
    let mut result = Vec::with_capacity(tasks.len());
    for task in tasks {
        result.push(
            shutdown_explicit(task, signal.clone()).await
        )
    }

    result
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

/// A combination of required tools for accessing and communicating with tasks, with additionall input value `A`.
pub struct ArgDuplexTask<T, O, A> where T: Send + 'static, O: Sized, A: Sized {
    join: JoinHandle<O>,
    sender: Sender<T>,
    receiver: Receiver<T>,
    _mark: PhantomData<A>
}
impl<T, O, A> TaskBasis for ArgDuplexTask<T, O, A> where T: Send + 'static, O: Send + 'static, A: Send + 'static {
    type Arg = (Sender<T>, Receiver<T>, A);
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
impl<T, O, A> RecvTaskBasis for ArgDuplexTask<T, O, A> where T: Send + 'static, O: Send + 'static, A: Send + 'static {
    fn receiver(&self) -> &Receiver<T> {
        &self.receiver
    }
    fn receiver_mut(&mut self) -> &mut Receiver<T> {
        &mut self.receiver
    }
}
impl<T, O, A> ArgDuplexTask<T, O, A> where T: Send + 'static, O: Send + 'static, A: Send + 'static  {
    /// Generates a `TaskHandle` from predetermined channels and join handle.
    pub fn new(join: JoinHandle<O>, sender: Sender<T>, receiver: Receiver<T>) -> Self {
        Self {
            join,
            sender,
            receiver,
            _mark: PhantomData
        }
    }

    pub fn start<F, Fut>(func: F, buffer_size: usize, extra: A) -> Self
        where F: FnOnce(<Self as TaskBasis>::Arg) -> Fut + Send + 'static,
        Fut: Future<Output = <Self as TaskBasis>::Output> + Send + 'static {
            let (my_sender, their_recv) = channel::<<Self as TaskBasis>::Msg>(buffer_size);
            let (their_sender, my_recv) = channel::<<Self as TaskBasis>::Msg>(buffer_size);

            let handle = tokio::spawn(async move {
                (func)((their_sender, their_recv, extra)).await
            });

            Self {
                join: handle,
                sender: my_sender,
                receiver: my_recv,
                _mark: PhantomData
            }
    }
}

/// A combination of required tools for accessing and communicating with tasks, with additionall input value `A`.
pub struct ArgSimplexTask<T, O, A> where T: Send + 'static, O: Send + 'static, A: Send + 'static {
    join: JoinHandle<O>,
    sender: Sender<T>,
    _mark: PhantomData<A>
}
impl<T, O, A> TaskBasis for ArgSimplexTask<T, O, A> where T: Send + 'static, O: Send + 'static, A: Send + 'static {
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
impl<T, O, A> ArgSimplexTask<T, O, A> where T: Send + 'static, O: Send + 'static, A: Send + 'static {
    /// Generates a `TaskHandle` from predetermined channels and join handle.
    pub fn new(join: JoinHandle<O>, sender: Sender<T>) -> Self {
        Self {
            join,
            sender,
            _mark: PhantomData
        }
    }

    pub fn start<F, Fut>(func: F, buffer_size: usize, extra: A) -> Self
            where Self: Sized,
            F: FnOnce(Receiver<T>, A) -> Fut + Send + 'static,
            Fut: Future<Output = O> + Send + 'static {
        
        let (my_sender, their_recv) = channel::<T>(buffer_size);

        let handle = tokio::spawn(async move {
            (func)(their_recv, extra).await
        });

        Self {
            join: handle,
            sender: my_sender,
            _mark: PhantomData
        }
    }
}