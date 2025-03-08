use tokio::sync::mpsc::{Sender, Receiver};

use crate::orchestra::ConsoleComm;

pub async fn console_entry(orch: Sender<ConsoleComm>, recv: Receiver<ConsoleComm>) {

}