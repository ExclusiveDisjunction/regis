use tokio::sync::mpsc::{Sender, Receiver};

use crate::orchestra::Comm;

pub async fn metrics_entry(orch: Sender<Comm>, recv: Receiver<Comm>) {

}