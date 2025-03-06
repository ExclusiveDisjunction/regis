use tokio::sync::mpsc::{Sender, Receiver};

use crate::orchestra::Comm;

pub async fn client_entry(orch: Sender<Comm>, recv: Receiver<Comm>) {

}