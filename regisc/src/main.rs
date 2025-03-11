use common::log::logging;
use common::{log_info, log_warning};
use common::msg::{Acknoledgement, send_request, decode_response};
use regisd_com::loc::SERVER_COMM_PATH;
use regisd_com::msg::{AuthenticateRequest, ShutdownRequest, UpdateConfigRequest};

use serde_json::{from_str, to_string};

use tokio::net::UnixStream;

#[tokio::main]
pub async fn main() {
    
}