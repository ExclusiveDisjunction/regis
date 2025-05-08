use std::{
    fmt::{
        Display,
        Debug
    },
    marker::PhantomData,
    sync::Arc
};

use futures::future::BoxFuture;
use tokio::{
    select,
    sync::mpsc::{
        channel, 
        error::SendError, 
        Receiver, 
        Sender
    }, 
    task::{
        JoinError,
        JoinHandle
    }
};

use crate::{log_warning, log_error, log_critical, log_debug};

/// A specific object that has a "kill" message value, such that if passed into a thread that listens to this message kind, it will stop executing.
pub trait KillMessage : Send + Sized{
    fn kill() -> Self;
}
/// A specific object that has a "poll"  message value, such that if passed into a thread that listens to this message kind, it will ignore it. 
pub trait PollableMessage : Send + Sized {
    fn poll() -> Self;
}

/// Represents a status that can be determined for restarting.
pub trait RestartStatusBase: Send + Debug {
    /// Determines if a task can be restarted, with no predcondition. 
    fn is_restartable(&self) -> bool;
    /// Determines if the task can be restarted, but has a precondition for doing so. Note that if this is true, `is_restartable` is not required to be true. 
    fn conditionally_restartable(&self) -> bool;
    /// IF the task should not be restarted.
    fn is_non_restartable(&self) -> bool {
        !self.is_restartable() && !self.conditionally_restartable()
    }
}

impl<T, E> RestartStatusBase for Result<T, E> where T: RestartStatusBase, E: Debug + Send {
    fn is_restartable(&self) -> bool {
        match self {
            Ok(v) => v.is_restartable(),
            Err(_) => false
        }
    }
    fn conditionally_restartable(&self) -> bool {
        match self {
            Ok(v) => v.conditionally_restartable(),
            Err(_) => false
        }
    }
}

/// Representation of the success/failures of restarting a task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartStatus<T: RestartStatusBase> {
    Ok,
    WasDead(T),
    TriesExceeded,
    Argument
}
impl<T> Display for RestartStatus<T> where T: RestartStatusBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ok => "Ok".to_owned(),
                Self::WasDead(v) => format!("was dead '{:?}'", v),
                Self::TriesExceeded => "restart tries exceeded".to_owned(),
                Self::Argument => "invalid argument passed".to_owned()
            }
        )
    }
}
impl<T> RestartStatus<T> where T: RestartStatusBase {
    /// Determines if the status is the ok variant. 
    pub fn is_ok(&self) -> bool {
        match self {
            Self::WasDead(v) => v.is_restartable(),
            Self::Ok => true,
            _ => false
        }
    }
    /// Determines if the status is an error variant.
    pub fn is_err(&self) -> bool {
        !matches!(self, Self::Ok)
    }
    /// Determines if the thread was restarted (warning value)
    pub fn was_restarted(&self) -> bool {
        match self {
            Self::WasDead(v) => v.is_restartable(),
            _ => false
        }
    }
    pub fn can_be_restarted(&self) -> bool {
        match self {
            Self::Ok => true,
            Self::Argument | Self::TriesExceeded => false,
            Self::WasDead(v) => v.is_restartable()
        }
    }

    /// Writes to the logger depending on the result of the poll. Note that 'Ok' values are only printed in debug mode.
    pub fn log_event(&self, name: &str) -> bool {
        match self {
            Self::Ok => log_debug!("Poll of thread '{}' was ok", name),
            Self::WasDead(v) => {
                if v.is_restartable() {
                    log_warning!("Poll of thread '{}' determined it was dead, but was successfully restarted.", name)
                }
                else {
                    log_critical!("Poll of thread '{}' determined it was dead, and could not be restarted.", name)
                }
            },
            Self::TriesExceeded => log_critical!("Poll of thread '{}' determined it was dead, but cannot be restarted.", name),
            Self::Argument => log_error!("Poll of thread '{}' determined that an argument passed into it was invalid, and could not be restarted.", name)
        }

        self.is_ok()
    }
}

/// An abstraction over communication channel(s) and a join handle. It handles the spawning and resulting of different async tasks.
pub trait TaskBasis {
    type Msg: Send + 'static;
    type Output: Send + 'static;
    type Arg: Send + 'static;

    /// Retreives the join handle from the task as a reference.
    fn join_handle(&self) -> &Option<JoinHandle<Self::Output>>;
    fn join_handle_mut(&mut self) -> &mut Option<JoinHandle<Self::Output>>;
    /// Consumes this object and returns the join handle from within.
    fn join_handle_owned(self) -> Option<JoinHandle<Self::Output>>;
    /// Gets the sender used by this task.
    fn sender(&self) -> &Sender<Self::Msg>;

    /// Determines if the holding task is still running.
    fn is_running(&self) -> bool {
        match self.join_handle() {
            Some(v) => v.is_finished(),
            None => false
        }
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
        fn start<F, Fut>(func: Arc<F>, buffer_size: usize) -> Self where
            Self: Sized,
            F: Fn(Self::Arg) -> Fut + Send + Sync + 'static + ?Sized,
            Fut: Future<Output = Self::Output> + Send + 'static;

        fn start_owned<F, Fut>(func: F, buffer_size: usize) -> Self where 
            Self: Sized,
            F: Fn(Self::Arg) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Self::Output> + Send + 'static {
                let arc = Arc::new(func);

                Self::start(arc, buffer_size)
            }
}

/// Attempts to restart the current task.
/// If the `buffer_size` is zero, it will return `RestartStatus::Argument`.
/// Otherwise, it will restart the task, if it was dead. If it was dead, this will return `RestartStatus::WasDead`, otherwise, `RestartStatus::Ok`.
pub async fn restart<T, F, Fut>(task: &mut T, func: Arc<F>, buffer_size: usize) -> RestartStatus<T::Output> where
    T: StartableTask,
    F: Fn(T::Arg) -> Fut + Send + Sync + 'static + ?Sized,
    Fut: Future<Output = T::Output> + Send + 'static,
    T::Output: RestartStatusBase {
        if task.is_running() {
            RestartStatus::Ok
        }
        else if buffer_size == 0 {
            RestartStatus::Argument
        }
        else {
            let handle_mut = task.join_handle_mut();
            let handle = handle_mut.take();
            let old_result = match handle {
                Some(v) => {
                    v.await
                }
                None => return RestartStatus::Argument
            };

            let dead_status = match old_result {
                Ok(v) => v,
                Err(_) => return RestartStatus::Argument
            };

            if dead_status.is_restartable() {
                *task = T::start(func, buffer_size);
            }
            RestartStatus::WasDead(dead_status)
        }
}

pub async fn restart_owned<T, F, Fut>(task: &mut T, func: F, buffer_size: usize) -> RestartStatus<T::Output> where
    T: StartableTask,
    F: Fn(T::Arg) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = T::Output> + Send + 'static,
    T::Output: RestartStatusBase {
        let arc = Arc::new(func);

        restart(task, arc, buffer_size).await 
}

/// A wrapper around a task that can be restarted. It requires that the task is a `StartableTask`. It will keep track of how many times the task will be restarted.
pub struct RestartableTask<Task>
    where Task: StartableTask,
    Task::Output: RestartStatusBase {
        task: Task,
        func: Arc<dyn Fn(Task::Arg) -> BoxFuture<'static, Task::Output> + Send + Sync>,
        restart_count: u8,
        max_restart: u8,
        buff_size: usize
}
impl<Task> TaskBasis for RestartableTask<Task>
    where Task: StartableTask,
    Task::Output: RestartStatusBase  {
        type Arg = Task::Arg;
        type Msg = Task::Msg;
        type Output = Task::Output;

        fn join_handle(&self) -> &Option<JoinHandle<Self::Output>> {
            self.task.join_handle()
        }
        fn join_handle_mut(&mut self) -> &mut Option<JoinHandle<Self::Output>> {
            self.task.join_handle_mut()
        }
        fn join_handle_owned(self) -> Option<JoinHandle<Self::Output>> {
            self.task.join_handle_owned()
        }
        fn sender(&self) -> &Sender<Self::Msg> {
            self.task.sender()
        }
}
impl<Task> RecvTaskBasis for RestartableTask<Task>
    where Task: StartableTask + RecvTaskBasis,
    Task::Output: RestartStatusBase {
        fn receiver(&self) -> &Receiver<Self::Msg> {
            self.task.receiver()
        }
        fn receiver_mut(&mut self) -> &mut Receiver<Self::Msg> {
            self.task.receiver_mut()
        }
    }
impl<Task> RestartableTask<Task>
    where Task: StartableTask,
    Task::Output: RestartStatusBase {

        /// Establishes a restartable task with a specific `max_restart` value.
        /// # Panics
        /// If `max_restart` or `buffer_size` is `0`, this will panic.
        pub fn start<F, Fut>(func: F, buffer_size: usize, max_restart: u8) -> Self 
            where F: Fn(Task::Arg) -> Fut + Send + Sync + 'static, 
            Fut: Future<Output=Task::Output> + Send + 'static {
                if max_restart == 0 || buffer_size == 0 {
                    panic!("The max restart cannot be zero.");
                }

                let func = Arc::new(func);

                let task = Task::start(Arc::clone(&func), buffer_size);

                Self {
                    task,
                    func: Arc::new(move |arg: Task::Arg| {
                        let fut = (func)(arg);
                        Box::pin(fut)
                    }),
                    restart_count: 0,
                    buff_size: buffer_size,
                    max_restart
                }
        }

        /// Attempts to restart the current task.
        /// If the `buffer_size` is zero, it will return `RestartStatus::Argument`.
        /// If the amount of restarts is greater than or requal to the `max_restart`, it will return `RestartStatus::TriesExceeded`
        /// Otherwise, it will restart the task, if it was dead. If it was dead, this will return `RestartStatus::WasDead`, otherwise, `RestartStatus::Ok`.
        pub async fn restart(&mut self) -> RestartStatus<Task::Output> {
                if self.restart_count + 1 == self.max_restart {
                    RestartStatus::TriesExceeded
                }
                else {
                    self.restart_count += 1;
                    restart(&mut self.task, Arc::clone(&self.func), self.buff_size).await
                }
        }

        /// Performs a poll on the task. If the poll fails, it will restart the task according to `self.restart`.
        pub async fn poll_and_restart(&mut self) -> RestartStatus<Task::Output> where
            Task::Msg: PollableMessage {

                if !poll(&mut self.task).await {
                    self.restart().await 
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
pub async fn join<Task>(handle: Task) -> Result<Option<Task::Output>, JoinError> where Task: TaskBasis {
    match handle.join_handle_owned() {
        Some(v) => v.await.map(Option::Some),
        None => Ok(None)
    }
}
/// If the task is currently running, it will send the 'T::kill()` value. After that, if the send was ok, it will join the handle. Note that errors are not considered nor recorded.
pub async fn shutdown<Task>(handle: Task) -> Result<Option<Task::Output>, JoinError> where Task: TaskBasis, Task::Msg: KillMessage {
    shutdown_explicit(handle, Task::Msg::kill()).await
}
/// If the task is currently running, it will send the `signal` value. After that, if the send was ok, it will join the handle. Note that errors are not considered nor recorded.
pub async fn shutdown_explicit<Task>(handle: Task,  signal: Task::Msg) -> Result<Option<Task::Output>, JoinError> where Task: TaskBasis{
    if handle.is_running() && send(&handle, signal).await.is_ok() {
        let join_handle = match handle.join_handle_owned() {
            Some(v) => v,
            None => return Ok(None)
        };

        let abort_handle = join_handle.abort_handle();

        select! { 
            v = join_handle => {
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
    if !handle.is_running() {
        false
    }
    else {
        send(handle, Task::Msg::poll()).await.is_ok()
    }
}

/// A combination of required tools for accessing and communicating with tasks.
pub struct DuplexTask<T, O> where T: Send + 'static, O: Sized {
    join: Option<JoinHandle<O>>,
    sender: Sender<T>,
    receiver: Receiver<T>
}
impl<T, O> TaskBasis for DuplexTask<T, O> where T: Send + 'static, O: Send + 'static {
    type Arg = (Sender<T>, Receiver<T>);
    type Msg = T;
    type Output = O;
    fn join_handle(&self) -> &Option<JoinHandle<O>> {
        &self.join
    }
    fn join_handle_mut(&mut self) -> &mut Option<JoinHandle<Self::Output>> {
        &mut self.join
    }
    fn join_handle_owned(self) -> Option<JoinHandle<O>> {
        self.join
    }
    fn sender(&self) -> &Sender<T> {
        &self.sender
    }
}
impl<T, O> StartableTask for DuplexTask<T, O> where T: Send + 'static, O: Send + 'static {
    fn start<F, Fut>(func: Arc<F>, buffer_size: usize) -> Self
            where Self: Sized,
            F: Fn(Self::Arg) -> Fut + Send + Sync + 'static + ?Sized,
            Fut: Future<Output = Self::Output> + Send + 'static {
        
        let (my_sender, their_recv) = channel::<Self::Msg>(buffer_size);
        let (their_sender, my_recv) = channel::<Self::Msg>(buffer_size);

        let handle = tokio::spawn(async move {
            (func)((their_sender, their_recv)).await
        });

        Self {
            join: Some(handle),
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
            join: Some(join),
            sender,
            receiver
        }
    }
}

/// A combination of required tools for accessing and communicating with tasks.
pub struct SimplexTask<T, O> where T: Send + 'static, O: Send + 'static {
    join: Option<JoinHandle<O>>,
    sender: Sender<T>,
}
impl<T, O> TaskBasis for SimplexTask<T, O> where T: Send + 'static, O: Send + 'static {
    type Arg = Receiver<T>;
    type Msg = T;
    type Output = O;

    fn join_handle(&self) -> &Option<JoinHandle<O>> {
        &self.join
    }
    fn join_handle_mut(&mut self) -> &mut Option<JoinHandle<Self::Output>> {
        &mut self.join
    }
    fn join_handle_owned(self) -> Option<JoinHandle<O>> {
        self.join
    }
    fn sender(&self) -> &Sender<T> {
        &self.sender
    }
}
impl<T, O> StartableTask for SimplexTask<T, O> where T: Send + 'static, O: Send + 'static {
    fn start<F, Fut>(func: Arc<F>, buffer_size: usize) -> Self
            where Self: Sized,
            F: Fn(Receiver<T>) -> Fut + Send + Sync + 'static + ?Sized,
            Fut: Future<Output = O> + Send + 'static {
        
        let (my_sender, their_recv) = channel::<T>(buffer_size);

        let their_func = Arc::clone(&func);
        let handle = tokio::spawn(async move {
            (their_func)(their_recv).await
        });

        Self {
            join: Some(handle),
            sender: my_sender
        }
    }
}
impl<T, O> SimplexTask<T, O> where T: Send + 'static, O: Send + 'static{
    /// Generates a `TaskHandle` from predetermined channels and join handle.
    pub fn new(join: JoinHandle<O>, sender: Sender<T>) -> Self {
        Self {
            join: Some(join),
            sender
        }
    }
}

/// A combination of required tools for accessing and communicating with tasks, with additionall input value `A`.
pub struct ArgDuplexTask<T, O, A> where T: Send + 'static, O: Sized, A: Sized {
    join: Option<JoinHandle<O>>,
    sender: Sender<T>,
    receiver: Receiver<T>,
    _mark: PhantomData<A>
}
impl<T, O, A> TaskBasis for ArgDuplexTask<T, O, A> where T: Send + 'static, O: Send + 'static, A: Send + 'static {
    type Arg = (Sender<T>, Receiver<T>, A);
    type Msg = T;
    type Output = O;

    fn join_handle(&self) -> &Option<JoinHandle<O>> {
        &self.join
    }
    fn join_handle_mut(&mut self) -> &mut Option<JoinHandle<Self::Output>> {
        &mut self.join
    }
    fn join_handle_owned(self) -> Option<JoinHandle<O>> {
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
            join: Some(join),
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
                join: Some(handle),
                sender: my_sender,
                receiver: my_recv,
                _mark: PhantomData
            }
    }
}

/// A combination of required tools for accessing and communicating with tasks, with additionall input value `A`.
pub struct ArgSimplexTask<T, O, A> where T: Send + 'static, O: Send + 'static, A: Send + 'static {
    join: Option<JoinHandle<O>>,
    sender: Sender<T>,
    _mark: PhantomData<A>
}
impl<T, O, A> TaskBasis for ArgSimplexTask<T, O, A> where T: Send + 'static, O: Send + 'static, A: Send + 'static {
    type Arg = Receiver<T>;
    type Msg = T;
    type Output = O;

    fn join_handle(&self) -> &Option<JoinHandle<O>> {
        &self.join
    }
    fn join_handle_mut(&mut self) -> &mut Option<JoinHandle<Self::Output>> {
        &mut self.join
    }
    fn join_handle_owned(self) -> Option<JoinHandle<O>> {
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
            join: Some(join),
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
            join: Some(handle),
            sender: my_sender,
            _mark: PhantomData
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug)]
    pub struct TestRestart;
    impl RestartStatusBase for TestRestart {
        fn is_restartable(&self) -> bool {
            true
        }
        fn conditionally_restartable(&self) -> bool {
            false
        }
    }

    async fn test_func(_recv: Receiver<()>) -> TestRestart {
        println!("I did something!");
        TestRestart
    }

    #[test]
    fn test_func_syntax() {
        let _task = SimplexTask::start_owned(test_func, 10);

        let _rtask: RestartableTask<SimplexTask<_, _>> = RestartableTask::start(test_func, 10, 4);
    }
}