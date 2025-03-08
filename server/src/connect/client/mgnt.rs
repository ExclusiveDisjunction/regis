use tokio::sync::mpsc::{Sender, Receiver};

use crate::orchestra::SimpleComm;

pub async fn client_entry(orch: Sender<SimpleComm>, recv: Receiver<SimpleComm>) {

}