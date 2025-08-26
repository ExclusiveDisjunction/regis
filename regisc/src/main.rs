use std::process::ExitCode;

#[cfg(unix)]
pub mod work;

#[cfg(feature="gui")]
pub mod gui;

pub mod core;
pub mod cli;

fn main() -> Result<(), ExitCode> {
    if cfg!(target_os="windows") {
        println!("Unfortunatley, this software is not offered on windows software. Please compile and run for linux, macOS, or BSD.");
        std::process::exit(1);
    }

    // Since this requires named pipes, this can only run on unix. That macro ensures that.

    #[cfg(unix)]
    work::entry()
}