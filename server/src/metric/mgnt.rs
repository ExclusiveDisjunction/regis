use tokio::sync::mpsc::Receiver;

use crate::message::SimpleComm;

pub async fn metrics_entry(recv: Receiver<SimpleComm>) {

}