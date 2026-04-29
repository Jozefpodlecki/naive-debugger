use std::{collections::HashMap, path::Path};
use windows_sys::Win32::Foundation::HANDLE;

use crate::{*, event::DebugEvent, win32::*};

#[derive(Clone, Copy, Debug)]
pub struct HardwareBreakpoint {
    pub enabled: bool,
    pub address: usize,
    pub bp_type: u32,
    pub size: u32,
}

pub trait Debugger {
    fn next_event(&mut self) -> Result<DebugEvent, DebugError>;
    fn continue_event(&mut self, context: DebugContext, status: ContinueStatus) -> Result<(), DebugError>;
}

#[derive(Clone, Copy)]
pub struct DebuggerOptions {
    pub single_step: bool,
}

impl Default for DebuggerOptions {
    fn default() -> Self {
        Self {
            single_step: false,
        }
    }
}

pub struct WindowsDebugger {
    process: ProcessHandle,
    threads: HashMap<u32, ThreadInfo>,
    hwbps: [Option<HardwareBreakpoint>; 4],
    options: DebuggerOptions,
}

impl Debugger for WindowsDebugger {
    fn next_event(&mut self) -> Result<DebugEvent, DebugError> {

        let event = wait_event(1000)?;
        let event = DebugEvent::decode(&event, self.process.handle);

        match &event.kind {
             DebugEventKind::Exception(e) => {
                println!("EXCEPTION: 0x{:x} @ {}", e.code, e.address);
            }
            DebugEventKind::CreateProcess(info) => {
                
            }
            DebugEventKind::LoadDll(info) => {
                
            }
            DebugEventKind::CreateThread(info) => {
                self.threads.insert(event.context.thread_id, ThreadInfo {
                    thread_id: event.context.thread_id,
                    handle: info.handle,
                    start_address: info.start_address.0,
                    teb: info.teb,
                });
            }
            DebugEventKind::ExitThread(_) => {
                self.threads.remove(&event.context.thread_id);
            }
            _ => {}
        }

        Ok(event)
    }

    fn continue_event(
        &mut self,
        context: DebugContext,
        status: ContinueStatus,
    ) -> Result<(), DebugError> {

        if self.options.single_step {
            // if ExitThread we should not enter here

            if let Some(thread) = self.threads.get(&context.thread_id) {
                println!("[1] setting next step for thread {}", context.thread_id);
                thread.enable_single_step()?;
            }
            else {
                self.threads.insert(context.thread_id, ThreadInfo::from_id(context.thread_id)?);

                let thread = unsafe { self.threads.get(&context.thread_id).unwrap_unchecked() };
                println!("[2] setting next step for thread {}", context.thread_id);
                thread.enable_single_step()?;
            }
        }
        
        continue_event(
            context.process_id,
            context.thread_id,
            status.as_win32());
        Ok(())
    }
    
}

impl WindowsDebugger {
     pub fn set_hardware_breakpoint(
        &mut self,
        thread_id: u32,
        address: usize,
        index: Option<usize>, // 0..3
        kind: HardwareBreakpointType,
        size: HardwareBreakpointSize,
    ) -> Result<(), DebugError> {

        let thread = self
            .threads
            .get_mut(&thread_id)
            .ok_or_else(|| DebugError::Other("thread not found".into()))?;

        let slot = match index {
            Some(i) if i < 4 => i,
            Some(_) => return Err(DebugError::Other("invalid breakpoint slot".into())),
            None => {
                // naive allocator: first free slot
                let used = self.hwbps.iter().enumerate().find_map(|(i, bp)| {
                    bp.as_ref().map(|_| i)
                });

                match used {
                    Some(i) if i < 4 => i,
                    _ => return Err(DebugError::Other("no free hw breakpoint slot".into())),
                }
            }
        };

        self.hwbps[slot] = Some(HardwareBreakpoint {
            enabled: true,
            address,
            bp_type: match kind {
                HardwareBreakpointType::Execute => 0,
                HardwareBreakpointType::Write => 1,
                HardwareBreakpointType::ReadWrite => 2,
            },
            size: match size {
                HardwareBreakpointSize::U1 => 1,
                HardwareBreakpointSize::U2 => 2,
                HardwareBreakpointSize::U4 => 4,
                HardwareBreakpointSize::U8 => 8,
            },
        });

        thread.set_hardware_breakpoint(
            address,
            slot,
            kind,
            size,
        )?;

        Ok(())
    }

    pub fn spawn_with_options<R: AsRef<Path>>(
        path: R,
        options: DebuggerOptions,
    ) -> Result<Self, DebugError> {
        let process = CreateProcessBuilder::new()
            .with_path(path)
            .with_debug_only_this_process()
            .spawn()?;

        Ok(Self {
            process,
            threads: HashMap::new(),
            hwbps: [None, None, None, None],
            options,
        })
    }

    pub fn spawn<R: AsRef<Path>>(path: R) -> Result<Self, DebugError> {
        Self::spawn_with_options(path, DebuggerOptions::default())
    }

    pub fn attach(pid: u32) -> Result<Self, DebugError> {
        let flags = ProcessAccessFlags::ALL_ACCESS;
        let process = CreateProcessBuilder::attach(pid, flags)?;

        Ok(Self {
            process,
            threads: HashMap::new(),
            hwbps: [None, None, None, None],
            options: Default::default()
        })
    }

}