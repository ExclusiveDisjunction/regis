use std::process::ExitCode;

use common::log_critical;

use crate::err::WEIRD_ERR_EXIT;

pub fn gui_entry() -> Result<(), ExitCode> {
    log_critical!("As of this moment, the GUI of this software is not offered yet. Run with --no-gui to enter CLI mode.");
    Err(ExitCode::from(WEIRD_ERR_EXIT))
}