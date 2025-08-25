use std::fmt::Display;

/// A representation of communication between the `Orchestrator` and the client worker tasks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimpleComm {
    /// A message from the orchestrator to reload configuration.
    ReloadConfiguration,
}
impl Display for SimpleComm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::ReloadConfiguration => "configuration reload",
            }
        )
    }
}

/// A representation of communication between the `Orchestrator` and the console worker tasks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleComm {
    /// A command to tell the console to approve the authentications that are pending.
    Auth,

    /// A message to the ochestrator to shutdown all tasks.
    SystemShutdown,
    //// A message to the ochestrator to tell other tasks to reload configuration.
    ReloadConfiguration,
}
impl Display for ConsoleComm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
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
