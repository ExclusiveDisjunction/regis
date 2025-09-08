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
    /// Instructs the orch to shut down Regisd
    Shutdown,
    /// Instructs the orch to reload the configuration and update all threads underneath it.
    /// The boolean parameter states if Orch should load from a file. When true, the orch will try to read from the config file.
    ConfigReload(bool)
}
impl Display for ConsoleComm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Shutdown => "system shutdown",
                Self::ConfigReload(_) => "configuration reload",
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
