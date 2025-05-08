use std::error::Error;
use std::fmt::{Display, Debug};
use std::process::ExitCode;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DaemonFailure {
    UnexepctedError = 1,
    RuntimeFailure = 2,
    SetupDirectoryError = 3,
    LoggerError = 4,
    SetupStreamError = 5,
    DaemonizeFailure = 6,   
    IOError = 7,
    ConfigurationError = 8,
    AuthenicationError = 9,
    SignalFailure = 10,
}
impl Debug for DaemonFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::UnexepctedError => "Unexpected Error",
                Self::RuntimeFailure => "Runtime Failure",
                Self::SetupDirectoryError => "Startup Directories could not be made",
                Self::LoggerError => "The logger could not be created",
                Self::SetupStreamError => "The stderr or stdout streams could not be bound",
                Self::DaemonizeFailure => "The software was requested to daemonize, but was unable to do so",
                Self::IOError => "An IO error occured",
                Self::ConfigurationError => "The configuration is invalid & could not be parsed",
                Self::AuthenicationError => "The authentication server failed",
                Self::SignalFailure => "Bindings to OS signals could not be made"
            }
        )
    }
}
impl Display for DaemonFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self as &dyn Debug).fmt(f)
    }
}
impl Error for DaemonFailure {

}
impl From<DaemonFailure> for ExitCode {
    fn from(value: DaemonFailure) -> Self {
        (value as u8).into()
    }
}