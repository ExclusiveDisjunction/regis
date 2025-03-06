use tokio::sync::mpsc::{Sender, Receiver};

use crate::orchestra::Comm;

pub async fn console_entry(orch: Sender<Comm>, recv: Receiver<Comm>) {

}