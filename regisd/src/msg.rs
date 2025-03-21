use std::fmt::Display;

use exdisj::task_util::{KillMessage, PollableMessage, RestartStatusBase};

use regisd_com::msg::ConsoleRequests;

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
impl Display for SimpleComm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Poll => "poll",
                Self::Kill => "kill",
                Self::ReloadConfiguration => "configuration reload",
            }
        )
    }
}

/// A representation of communication between the `Orchestrator` and the console worker tasks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleComm {
    /// A request to see if the thread in question is working fine.
    Poll,
    /// A command to tell that task to stop executing.
    Kill,
    /// A command to tell the console to approve the authentications that are pending.
    Auth,

    /// A message to the ochestrator to shutdown all tasks.
    SystemShutdown,
    //// A message to the ochestrator to tell other tasks to reload configuration.
    ReloadConfiguration,
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
impl From<ConsoleRequests> for ConsoleComm {
    fn from(value: ConsoleRequests) -> Self {
        match value {
            ConsoleRequests::Auth => Self::Auth,
            ConsoleRequests::Config => Self::ReloadConfiguration,
            ConsoleRequests::Shutdown => Self::SystemShutdown,
            ConsoleRequests::Poll => Self::Poll
        }
    }
}
impl Display for ConsoleComm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Poll => "poll",
                Self::Kill => "kill",
                Self::SystemShutdown => "system shutdown",
                Self::ReloadConfiguration => "configuration reload",
                Self::Auth => "authentication approved"
            }
        )
    }
}

/// A simple enum that shows some common reasons why worker threads of the Orch would fail.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WorkerTaskResult {
    /*
    Ok,
    Configuration,
    DoNotReboot,
    Sockets,
    Failure,
    ImproperShutdown,
    */

    /// The configuration could not be read
    ConfigError,
    /// The network that the task uses could not be connected to, or messages could not be checked.
    NetworkFail,
    /// A worker task had a failure that caused the current task to fail.
    WorkerFail,
    /// A shutdown of the task did not go succesfully
    ImproperShutdown,
    /// A failure that could not be described directly, but should not be restarted.
    ComplexFail,
    /// A failure to a simple reason that can be restarted. 
    SimpleFail,
}
impl Display for WorkerTaskResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::ConfigError => "configuration issue",
                Self::NetworkFail => "network failure",
                Self::WorkerFail => "a worker task failed",
                Self::ImproperShutdown => "the task was not able to shutdown properly",
                Self::ComplexFail => "a complex failure",
                Self::SimpleFail => "a simple failure",
            }
        )
    }
}
impl RestartStatusBase for WorkerTaskResult {
    fn is_restartable(&self) -> bool {
        matches!(self, Self::SimpleFail | Self::NetworkFail | Self::WorkerFail | Self::ImproperShutdown)
    }
    fn conditionally_restartable(&self) -> bool {
        matches!(self, Self::ConfigError)
    }
}
impl WorkerTaskResult {
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }
    pub fn is_err(&self) -> bool {
        !self.is_ok()
    }
}
