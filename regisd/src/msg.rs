use std::fmt::Display;

use exdisj::task_util::{KillMessage, PollableMessage};
use exdisj::task_util::RestartStatusBase;
use common::msg::ConsoleRequests;

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
    Ok,
    Configuration,
    DoNotReboot,
    Sockets,
    Failure,
    ImproperShutdown,
}
impl Display for WorkerTaskResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ok => "ok",
                Self::Configuration => "configuration issue",
                Self::DoNotReboot => "error, unable to reboot",
                Self::Sockets => "sockets error",
                Self::Failure => "general failure, rebootable",
                Self::ImproperShutdown => "improper shutdown",
            }
        )
    }
}
impl RestartStatusBase for WorkerTaskResult {
    fn is_restartable(&self) -> bool {
        matches!(self, Self::Ok | Self::ImproperShutdown )   
    }
    fn conditionally_restartable(&self) -> bool {
        matches!(self, Self::Sockets | Self::Configuration )
    }
}
