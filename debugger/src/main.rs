#![allow(unused)]

use std::{io, thread::sleep, time::Duration};

use clap::Parser;
use flexi_logger::{Duplicate, FileSpec, Logger};
use log::*;
use naive_debugger::*;

#[derive(clap::Parser)]
struct Args {
    #[arg(short, long)]
    input: String,
}

fn run() -> Result<(), DebugError> {
    let mut logger = Logger::try_with_str("info").unwrap();
    logger = logger.log_to_file(FileSpec::default());
    logger = logger.duplicate_to_stdout(Duplicate::All);
    logger.start().unwrap();
    let args = Args::parse();

    let current_exe = std::env::current_exe().unwrap();
    let exe_dir = current_exe.parent().unwrap();
    let exe_path = exe_dir.join(args.input);

    if !exe_path.exists() {
        return Err(DebugError::BinaryNotFound(exe_path));
    }
    
    let options = DebuggerOptions {
        single_step: true,
        breakpoint_on_every_instruction: true,
        wait_timeout: Duration::from_secs(200)
    };

    let mut debugger = WindowsDebugger
        ::spawn_with_options(exe_path, options)?;

    loop {
        let event = debugger.next_event()?;
        // info!("{event}");
        let context = event.context;

        let status = match event.kind {
            DebugEventKind::LoadDll(event) => {
                info!("{:?}", event);
                ContinueStatus::Continue
            }
            DebugEventKind::Breakpoint(event) => {

                let module = debugger.get_module_by_address(event.address).unwrap();

                info!("{} {:?} module={:?}", event.address, event.instr.asm(), module.name);
                debugger.remove_breakpoint(event.address)?;
                ContinueStatus::Continue
            }
            DebugEventKind::ExitProcess(_) => break,
            event => {
                info!("{:?}", event);
                ContinueStatus::Continue
            },
        };

        debugger.continue_event(context, status)?;

        // sleep(Duration::from_secs(2));
        // info!("Press Enter to continue...");
        // let _ = io::stdin().read_line(&mut String::new());
    }

    Ok(())
}

fn main() {


    if let Err(err) = run() {
        error!("{}", err);
    }
}
