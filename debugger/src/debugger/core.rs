use std::{collections::HashMap, fs::File, io::Read, path::Path, time::Duration};
use log::*;
use pelite::pe::{Pe, PeFile};
use windows_sys::Win32::{Foundation::HANDLE, System::Diagnostics::Debug::*};

use crate::{breakpoints::{Breakpoint, BreakpointManager}, disasm::{DebuggerInstruction, Disassembler}, event::DebugEvent, handlers::*, modules::{Module, ModulesManager}, win32::*, *};

pub trait Debugger {
    fn remove_breakpoint(&mut self, addr: Address) -> Result<(), DebugError>;
    fn next_event(&mut self) -> Result<DebugEvent, DebugError>;
    fn continue_event(&mut self, context: DebugContext, status: ContinueStatus) -> Result<(), DebugError>;
}

#[derive(Clone, Copy)]
pub struct DebuggerOptions {
    pub single_step: bool,
    pub breakpoint_on_every_instruction: bool,
    pub wait_timeout: Duration
}

impl Default for DebuggerOptions {
    fn default() -> Self {
        Self {
            single_step: false,
            breakpoint_on_every_instruction: false,
            wait_timeout: Duration::from_secs(5)
        }
    }
}

pub struct DebuggerContext<'a> {
    disasm: &'a Disassembler,
    process: &'a ProcessHandle,
    threads: &'a mut HashMap<u32, ThreadInfo>,
    breakpoints: &'a mut BreakpointManager,
    modules: &'a mut ModulesManager,
    pub options: &'a DebuggerOptions,
}

impl<'a> DebuggerContext<'a> {
    pub fn get_breakpoint(&self, addr: Address) -> Option<Breakpoint> {
        self.breakpoints.get(addr).cloned()
    }

    pub fn remove_breakpoint(&mut self, addr: Address) -> Result<(), DebugError> {
        debug!("Removing breakpoint at {}", addr);
        self.breakpoints.remove(addr, self.process.handle)
    }

    pub fn set_software_breakpoint(&mut self, addr: Address) -> Result<(), DebugError> {
        debug!("Setting software breakpoint at {}", addr);
        self.breakpoints.set(addr, breakpoints::BreakpointKind::Int3, self.process.handle)?;
        Ok(())
    }

    pub fn restore_instruction(&self, breakpoint: &Breakpoint) -> Result<DebuggerInstruction, DebugError> {

        let original_bytes = &breakpoint.original_bytes;
        let instr = self.disasm.decode_one(&original_bytes, breakpoint.address)?;
        let bytes_written = write_process_memory(
            self.process.handle,
            breakpoint.address,
            &breakpoint.original_bytes,
        )?;
            
        Ok(instr)
    }

    pub fn get_thread_mut(&mut self, id: u32) -> Result<&mut ThreadInfo, DebugError> {
        let thread = self.threads.get_mut(&id)
            .ok_or_else(|| DebugError::ThreadNotFound(id))?;

        Ok(thread)
    }

    pub fn insert_thread(&mut self, thread: ThreadInfo) {
        self.threads.insert(thread.thread_id, thread);
    }

    pub fn remove_thread(&mut self, id: &u32) {
        self.threads.remove(id);
    }

    pub fn insert_module(&mut self, module: Module) {
        self.modules.insert(module);
    }
}

pub struct WindowsDebugger {
    is_attached: bool,
    process: ProcessHandle,
    threads: HashMap<u32, ThreadInfo>,
    // hwbps: [Option<HardwareBreakpoint>; 4],
    options: DebuggerOptions,
    modules: ModulesManager,
    breakpoints: BreakpointManager,
    disasm: Disassembler,
}

impl Debugger for WindowsDebugger {
    fn next_event(&mut self) -> Result<DebugEvent, DebugError> {

        let event = wait_event(self.options.wait_timeout.as_millis() as u32)?;
        let event: DebugEvent = DebugEvent::decode(&event, self.process.handle);
        let debug_context = &event.context;

        let debugger_context = DebuggerContext {
            threads: &mut self.threads,
            breakpoints: &mut self.breakpoints,
            options: &self.options,
            disasm: &self.disasm,
            process: &self.process,
            modules: &mut self.modules
        };

        let new_event = match &event.kind {
            DebugEventKind::Exception(event) => handle_exception(debug_context, debugger_context, event)?,
            DebugEventKind::CreateProcess(info) => handle_create_process(debug_context, debugger_context, info)?,
            DebugEventKind::LoadDll(info) => handle_load_dll(debug_context, debugger_context, info)?,
            DebugEventKind::CreateThread(info) => handle_create_thread(debug_context, debugger_context, info)?,
            DebugEventKind::ExitThread(info) => handle_exit_thread(debug_context, debugger_context, info)?,
            _ => None
        };

        Ok(new_event.unwrap_or_else(|| event))
    }

    fn continue_event(
        &mut self,
        context: DebugContext,
        status: ContinueStatus,
    ) -> Result<(), DebugError> {

        // if self.options.single_step {
        //     // if ExitThread we should not enter here

        //     if let Some(thread) = self.threads.get(&context.thread_id) {
        //         println!("[1] setting next step for thread {}", context.thread_id);
        //         thread.enable_single_step()?;
        //     }
        //     else {
        //         self.threads.insert(context.thread_id, ThreadInfo::from_id(context.thread_id)?);

        //         let thread = unsafe { self.threads.get(&context.thread_id).unwrap_unchecked() };
        //         println!("[2] setting next step for thread {}", context.thread_id);
        //         thread.enable_single_step()?;
        //     }
        // }
        
        continue_event(
            context.process_id,
            context.thread_id,
            status.as_win32());
        Ok(())
    }
    
    fn remove_breakpoint(&mut self, addr: Address) -> Result<(), DebugError> {
        debug!("Removing breakpoint at {}", addr);
        self.breakpoints.remove(addr, self.process.handle)
    }
    
}

impl WindowsDebugger {
   
    pub fn get_module_by_address(&self, address: Address) -> Option<&Module> {
        self.modules.find_by_address(address)
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
            is_attached: false,
            process,
            threads: HashMap::new(),
            // hwbps: [None, None, None, None],
            options,
            modules: ModulesManager::new(),
            breakpoints: BreakpointManager::new(),
            disasm: Disassembler::new(),
        })
    }

    // pub fn spawn_with_agent<P: AsRef<Path>>(path: P) -> Result<Self, DebugError> {

    //     let process = CreateProcessBuilder::new()
    //         .with_path(path)
    //         .with_suspended(true)
    //         .spawn()?;
        
    //     let agent_path = std::env::current_exe()?
    //         .parent()
    //         .unwrap()
    //         .join("..\\agent\\debugger_agent.dll");
        
    //     inject_dll(process.handle, agent_path.to_str().unwrap())
    //         .map_err(|e| DebugError::Other(e))?;
        
    //     unsafe { ResumeThread(process.main_thread) };
        

    //     Ok(Self {
    //         process,
    //     })
    // }

    pub fn spawn<R: AsRef<Path>>(path: R) -> Result<Self, DebugError> {
        Self::spawn_with_options(path, DebuggerOptions::default())
    }

    pub fn attach_with_options(pid: u32, options: DebuggerOptions) -> Result<Self, DebugError> {
        let flags = ProcessAccessFlags::ALL_ACCESS;
        let process = CreateProcessBuilder::attach(pid, flags)?;

        let threads=  enumerate_threads(process.pid).unwrap();
        dbg!(&threads);

        for thread in threads {
            let code = thread.resume().unwrap();
            println!("thread_id={} resume={}", thread.thread_id, code);
        }

        Ok(Self {
            is_attached: true,
            process,
            threads: HashMap::new(),
            // hwbps: [None, None, None, None],
            options,
            modules: ModulesManager::new(),
            breakpoints: BreakpointManager::new(),
            disasm: Disassembler::new(),
        })
    }

    pub fn attach(pid: u32) -> Result<Self, DebugError> {
        Self::attach_with_options(pid, Default::default())
    }

}