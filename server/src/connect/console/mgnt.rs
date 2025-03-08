use tokio::sync::mpsc::{Sender, Receiver};

use crate::message::ConsoleComm;

pub async fn console_entry((orch, recv): (Sender<ConsoleComm>, Receiver<ConsoleComm>)) {

}