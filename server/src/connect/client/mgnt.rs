use tokio::sync::mpsc::Receiver;

use crate::message::SimpleComm;

pub async fn client_entry(recv: Receiver<SimpleComm>) {

}