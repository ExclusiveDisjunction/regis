use std::os::raw::c_uint;

pub use exdisj::error::*;

pub const CHECK_ERR_EXIT: u8 = 1;
pub const IO_ERR_EXIT: u8 = 2;
pub const LOG_ERR_EXIT: u8 = 3;
pub const NETWORK_ERR_EXIT: u8 = 4;
pub const WEIRD_ERR_EXIT: u8 = 5;
pub const CONFIG_ERR_EXIT: u8 = 6;
/// Describes when the user avoids doing something they have to. 
pub const AVOID_ERR_EXIT: u8 = 7;

pub static OK_MSG: c_uint = 0;
pub static NETWORK_FAIL: c_uint = 1;
pub static NETWORK_DISCONNECT: c_uint = 2;
pub static INVALID_ARG: c_uint = 3;
pub static IO_FAIL: c_uint = 4;
pub static NO_INFORMATION: c_uint = 5;


#[derive(Debug)]
pub enum IOCommError {
    IO(std::io::Error),
    Core(Error)
}
impl From<std::io::Error> for IOCommError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}
impl From<Error> for IOCommError {
    fn from(value: Error) -> Self {
        Self::Core(value)
    }
}