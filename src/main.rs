use std::{io, thread::sleep, time::Duration};

use naive_debugger::*;

fn main() {
    let exe_path = "C:\\repos\\rust_playground\\basic\\target\\debug\\basic.exe";
    
    let mut debugger = WindowsDebugger
        ::spawn_with_options(exe_path, DebuggerOptions { single_step: true })
        .unwrap();
    
    loop {
        let event = debugger.next_event().unwrap();
        let context = event.context;

        let status = match event.kind {
            DebugEventKind::LoadDll(event) => {
                println!("{:?}", event);
                ContinueStatus::Continue
            }
            DebugEventKind::ExitProcess(_) => break,
            event => {
                println!("{:?}", event);
                ContinueStatus::Continue
            },
        };

        if let Err(err) = debugger.continue_event(context, status) {
            println!("{}", err);
            break;
        }

        // sleep(Duration::from_secs(2));
        println!("Press Enter to continue...");
        let _ = io::stdin().read_line(&mut String::new());
    }
}
