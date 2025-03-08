use tokio::sync::mpsc::{Sender, Receiver};

use crate::orchestra::MetricComm;

pub async fn metrics_entry(orch: Sender<MetricComm>, recv: Receiver<MetricComm>) {

}