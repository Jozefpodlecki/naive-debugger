#![allow(unused)]

use std::{time::Duration, path::PathBuf};

use clap::Parser;
use flexi_logger::{Duplicate, FileSpec, Logger};
use log::*;
use naive_debugger::*;

#[derive(clap::Parser)]
#[command(name = "naive-debugger")]
#[command(about = "A Windows user-mode debugger", version = "0.1.0")]
enum Command {
    /// Spawn a new process
    Spawn {
        /// Path to the executable
        target: String,
        
        /// Timeout for WaitForDebugEvent in milliseconds
        #[arg(long, default_value = "10000")]
        timeout: u64,
        
        /// Enable single-step mode
        #[arg(long)]
        single_step: bool,
        
        /// Enable breakpoint on every instruction (tracer mode)
        #[arg(long)]
        trace: bool,
    },
    
    /// Attach to a running process by PID
    Attach {
        /// Process ID to attach to
        pid: u32,
        
        /// Timeout for WaitForDebugEvent in milliseconds
        #[arg(long, default_value = "10000")]
        timeout: u64,
        
        /// Enable single-step mode
        #[arg(long)]
        single_step: bool,
        
        /// Enable breakpoint on every instruction (tracer mode)
        #[arg(long)]
        trace: bool,
    },
    
    /// Attach to a running process by name (finds PID automatically)
    AttachName {
        /// Process name (e.g., "notepad.exe" or "notepad")
        name: String,
        
        /// Timeout for WaitForDebugEvent in milliseconds
        #[arg(long, default_value = "10000")]
        timeout: u64,
        
        /// Enable single-step mode
        #[arg(long)]
        single_step: bool,
        
        /// Enable breakpoint on every instruction (tracer mode)
        #[arg(long)]
        trace: bool,
    },
}

fn run() -> Result<(), DebugError> {
    let args = Command::parse();
    
    let mut logger = Logger::try_with_str("debug").unwrap();
    logger = logger.log_to_file(FileSpec::default());
    logger = logger.duplicate_to_stdout(Duplicate::All);
    logger.start().unwrap();
    
    match args {
        Command::Spawn { target, timeout, single_step, trace } => {
            let options = DebuggerOptions {
                single_step,
                breakpoint_on_every_instruction: trace,
                wait_timeout: Duration::from_millis(timeout),
            };
            
            let current_exe = std::env::current_exe().unwrap();
            let exe_dir = current_exe.parent().unwrap();
            let exe_path = if PathBuf::from(&target).exists() {
                PathBuf::from(&target)
            } else {
                let exe_name = if target.ends_with(".exe") {
                    target
                } else {
                    format!("{}.exe", target)
                };
                exe_dir.join(exe_name)
            };
            
            if !exe_path.exists() {
                return Err(DebugError::BinaryNotFound(exe_path));
            }
            
            info!("Spawning: {}", exe_path.display());
            let mut debugger = WindowsDebugger::spawn_with_options(exe_path, options)?;
            run_debugger_loop(&mut debugger, options)
        }
        
        Command::Attach { pid, timeout, single_step, trace } => {
            let options = DebuggerOptions {
                single_step,
                breakpoint_on_every_instruction: trace,
                wait_timeout: Duration::from_millis(timeout),
            };
            
            info!("Attaching to process PID={}", pid);
            info!("Wait={}ms", options.wait_timeout.as_millis());
            info!("Trace={}", options.breakpoint_on_every_instruction);
            let mut debugger = WindowsDebugger::attach_with_options(pid, options)?;
            run_debugger_loop(&mut debugger, options)
        }
        
        Command::AttachName { name, timeout, single_step, trace } => {
            let options = DebuggerOptions {
                single_step,
                breakpoint_on_every_instruction: trace,
                wait_timeout: Duration::from_millis(timeout),
            };
            
             match find_process_by_name(&name) {
                Some(pid) => {
                    info!("Found process '{}' with PID={}", name, pid);
                    info!("Attaching to process PID={}", pid);
                    info!("Wait={}ms", options.wait_timeout.as_millis());
                    info!("Trace={}", options.breakpoint_on_every_instruction);
                    let mut debugger = WindowsDebugger::attach_with_options(pid, options)?;
                    run_debugger_loop(&mut debugger, options)
                }
                None => {
                    error!("Could not find process with name \"{name}\"");
                    Ok(())
                }
            }
        }
    }
}

fn run_debugger_loop(
    debugger: &mut WindowsDebugger,
    options: DebuggerOptions,
) -> Result<(), DebugError> {
    loop {
        let event = debugger.next_event()?;
        let context = event.context;
        
        let status = match event.kind {
            DebugEventKind::LoadDll(event) => {
                info!("Loaded: {} at 0x{:X}", event.dll_name, event.base_address.0);
                ContinueStatus::Continue
            }
            DebugEventKind::Breakpoint(event) => {
                let module = debugger
                    .get_module_by_address(event.address)
                    .map(|m| m.name.as_str())
                    .unwrap_or("unknown");
                
                info!("Breakpoint at 0x{:X} [{}] - {}", event.address.0, module, event.instr.asm());
                debugger.remove_breakpoint(event.address)?;
                ContinueStatus::Continue
            }
            DebugEventKind::CreateProcess(event) => {
                let path = event.file_path;
                info!("Process created: {} entry at 0x{:X}", path, event.entry_point.0);
                ContinueStatus::Continue
            }
            DebugEventKind::CreateThread(event) => {
                info!("Thread created: TID={}", event.thread_id);
                ContinueStatus::Continue
            }
            DebugEventKind::ExitThread(event) => {
                info!("Thread exited: TID={}", event.thread_id);
                ContinueStatus::Continue
            }
            DebugEventKind::ExitProcess(event) => {
                info!("Process exited: code=0x{:X}", event.exit_code);
                break;
            }
            event => {
                info!("Other: {:?}", event);
                ContinueStatus::Continue
            }
        };
        
        debugger.continue_event(context, status)?;
    }
    
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        error!("{}", err);
    }
}