
use std::process::ExitCode;
use std::thread::{Thread, JoinHandle};
use std::sync::mpsc::channel;

use crate::config::CONFIG;

pub struct Orchestrator {

}
impl Orchestrator {
    pub fn initialize() -> Result<Self, ExitCode> {
        todo!()
    }

    pub fn run(&mut self) -> Result<(), ExitCode> {
        todo!()
    }
}