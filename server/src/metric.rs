pub mod collect;
pub mod io;

use tokio::sync::mpsc::Receiver;

use crate::message::{SimpleComm, WorkerTaskResult};

pub async fn metrics_entry(_recv: Receiver<SimpleComm>) -> WorkerTaskResult {    
    todo!()
}  