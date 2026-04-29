use naive_debugger::*;

fn main() {
    let exe_path = "C:\\repos\\rust_playground\\basic\\target\\debug\\basic.exe";
    
    let mut debugger = WindowsDebugger
        ::spawn_with_options(exe_path, DebuggerOptions { single_step: true })
        .unwrap();
    
    while let Ok(event) = debugger.next_event() {
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
    }
}
