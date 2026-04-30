use std::{collections::HashMap, fs::File, io::Read, path::Path};
use iced_x86::Instruction;
use pelite::pe::{Pe, PeFile};
use windows_sys::Win32::{Foundation::HANDLE, System::Diagnostics::Debug::*};

use crate::{breakpoints::{Breakpoint, BreakpointManager}, disasm::Disassembler, event::DebugEvent, handlers::*, win32::*, *};

#[derive(Clone, Copy, Debug)]
pub struct HardwareBreakpoint {
    pub enabled: bool,
    pub address: usize,
    pub bp_type: u32,
    pub size: u32,
}

pub struct Module {
    pub base_address: Address,
    pub entry_point: Option<Address>,
    pub name: String,
    pub size: usize,
    pub software_breakpoints: Vec<SoftwareBreakpointInfo>,
}

#[derive(Debug, Clone)]
pub struct SoftwareBreakpointInfo {
    pub address: Address,
    pub original_bytes: Vec<u8>,
    pub is_enabled: bool,
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

pub struct DebuggerContext<'a> {
    disasm: &'a Disassembler,
    process: &'a ProcessHandle,
    thread: &'a mut ThreadInfo,
    breakpoints: &'a mut BreakpointManager,
    pub breakpoint_on_every_instruction: bool
}

impl<'a> DebuggerContext<'a> {
    pub fn get_breakpoint(&self, addr: Address) -> Option<&Breakpoint> {
        self.breakpoints.get(addr)
    }

    pub fn remove_breakpoint(&mut self, addr: Address) -> Result<(), DebugError> {
        self.breakpoints.remove(addr, self.process.handle)
    }

    pub fn decrease_rip(&self) -> Result<(), DebugError> {
        self.thread.decrease_rip()
    }

    pub fn restore_instruction(&self, breakpoint: &Breakpoint) -> Result<Instruction, DebugError> {
        let addr = self.thread.get_rip()?;

        let bytes = breakpoint.original;

        let instr = self.disasm.decode_one(&bytes, addr)?;

        Ok(instr)
    }

    pub fn next_address(&self, instr: Instruction) -> Address {
        self.thread.get_rip().unwrap() + instr.len()
    }
}

pub struct WindowsDebugger {
    process: ProcessHandle,
    threads: HashMap<u32, ThreadInfo>,
    hwbps: [Option<HardwareBreakpoint>; 4],
    options: DebuggerOptions,
    modules: Vec<Module>,
    breakpoints: BreakpointManager,
    breakpoint_on_every_instruction: bool
}

impl Debugger for WindowsDebugger {
    fn next_event(&mut self) -> Result<DebugEvent, DebugError> {

        let event = wait_event(1000)?;
        let event = DebugEvent::decode(&event, self.process.handle);
        let debug_context = &event.context;

        match &event.kind {
             DebugEventKind::Exception(event) => {
                println!("EXCEPTION: 0x{:x} @ {}", event.code, event.address);

                let thread = self.threads.get_mut(&debug_context.thread_id)
                    .ok_or_else(|| DebugError::ThreadNotFound(debug_context.thread_id))?;
                let debugger_context = DebuggerContext {
                    thread,
                    breakpoints: &mut self.breakpoints,
                    breakpoint_on_every_instruction: self.breakpoint_on_every_instruction
                };

                handle_exception(debugger_context, event)?;
            }
            DebugEventKind::CreateProcess(info) => {
                handle_create_process(info)?;
            }
            DebugEventKind::LoadDll(info) => {
               handle_load_dll(info)?;

                // let mut module = self.create_module_from_path(info.base_address, &info.win32_path)?;

                // if let Some(entry_point) = module.entry_point {

                //     println!("[*] Setting breakpoint at DLL entry point: 0x{:x}", entry_point.0);
                //     // self.set_software_breakpoint(entry_point)?;
                //     self.breakpoints.set(addr, kind, callback, process);

                //     module.software_breakpoints.push(SoftwareBreakpointInfo {
                //         address: entry_point,
                //         original_bytes: vec![],
                //         is_enabled: true,
                //     });
                // }
                // else {
                //     println!("Could not set breakpoint for DLL {}", info.dll_name);
                // }
                
                // self.modules.push(module);
            }
            DebugEventKind::CreateThread(info) => {
                // self.threads.insert(event.context.thread_id, ThreadInfo {
                //     thread_id: event.context.thread_id,
                //     handle: info.handle,
                //     start_address: info.start_address.0,
                //     teb: info.teb,
                // });
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
    
}

impl WindowsDebugger {
    // fn create_module_from_path(&self, base_address: Address, dll_path: &str) -> Result<Module, DebugError> {

    //     println!("{}", dll_path);
        
    //     let mut file = File::open(dll_path).map_err(|e| DebugError::Other(format!("Failed to open {}: {}", dll_path, e)))?;
    //     let mut buffer = Vec::new();
    //     file.read_to_end(&mut buffer).map_err(|e| DebugError::Other(format!("Failed to read {}: {}", dll_path, e)))?;

    //     let pe = PeFile::from_bytes(&buffer).map_err(|e| DebugError::Other(format!("Invalid PE: {:?}", e)))?;
    //     let entry_point_rva = pe.optional_header().AddressOfEntryPoint as usize;
        
    //     let entry_point = if entry_point_rva != 0 {
    //         Some(Address(base_address.0 + entry_point_rva))
    //     } else {
    //         None
    //     };
        
    //     let size = pe.optional_header().SizeOfImage as usize;
        
    //     let name = std::path::Path::new(dll_path)
    //         .file_name()
    //         .unwrap_or_default()
    //         .to_string_lossy()
    //         .to_string();
        
    //     Ok(Module {
    //         base_address,
    //         entry_point,
    //         name,
    //         size,
    //         software_breakpoints: Vec::new(),
    //     })
    // }
    
    // pub fn set_software_breakpoint(process: ProcessHandle, address: Address) -> Result<(), DebugError> {
    //     unsafe {

    //         let mut original_byte = 0u8;
    //         let mut bytes_read = 0;
            
    //         let read_result = ReadProcessMemory(
    //             process.handle,
    //             address.0 as *const _,
    //             &mut original_byte as *mut _ as *mut _,
    //             1,
    //             &mut bytes_read,
    //         );
            
    //         if read_result == 0 {
    //             return Err(DebugError::Other("Failed to read original byte".into()));
    //         }
            
    //         let int3: u8 = 0xCC;
    //         let mut bytes_written = 0;
            
    //         let write_result = WriteProcessMemory(
    //             process.handle,
    //             address.0 as *mut _,
    //             &int3 as *const _ as *const _,
    //             1,
    //             &mut bytes_written,
    //         );
            
    //         if write_result == 0 {
    //             return Err(DebugError::Other("Failed to write INT3 breakpoint".into()));
    //         }
            
    //         Ok(())
    //     }
    // }

    //  pub fn set_hardware_breakpoint(
    //     &mut self,
    //     thread_id: u32,
    //     address: usize,
    //     index: Option<usize>, // 0..3
    //     kind: HardwareBreakpointType,
    //     size: HardwareBreakpointSize,
    // ) -> Result<(), DebugError> {

    //     let thread = self
    //         .threads
    //         .get_mut(&thread_id)
    //         .ok_or_else(|| DebugError::Other("thread not found".into()))?;

    //     let slot = match index {
    //         Some(i) if i < 4 => i,
    //         Some(_) => return Err(DebugError::Other("invalid breakpoint slot".into())),
    //         None => {
    //             // naive allocator: first free slot
    //             let used = self.hwbps.iter().enumerate().find_map(|(i, bp)| {
    //                 bp.as_ref().map(|_| i)
    //             });

    //             match used {
    //                 Some(i) if i < 4 => i,
    //                 _ => return Err(DebugError::Other("no free hw breakpoint slot".into())),
    //             }
    //         }
    //     };

    //     self.hwbps[slot] = Some(HardwareBreakpoint {
    //         enabled: true,
    //         address,
    //         bp_type: match kind {
    //             HardwareBreakpointType::Execute => 0,
    //             HardwareBreakpointType::Write => 1,
    //             HardwareBreakpointType::ReadWrite => 2,
    //         },
    //         size: match size {
    //             HardwareBreakpointSize::U1 => 1,
    //             HardwareBreakpointSize::U2 => 2,
    //             HardwareBreakpointSize::U4 => 4,
    //             HardwareBreakpointSize::U8 => 8,
    //         },
    //     });

    //     thread.set_hardware_breakpoint(
    //         address,
    //         slot,
    //         kind,
    //         size,
    //     )?;

    //     Ok(())
    // }

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
            modules: vec![],
            breakpoints: BreakpointManager::new(),
            breakpoint_on_every_instruction: false
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
            options: Default::default(),
            modules: vec![],
            breakpoints: BreakpointManager::new(),
            breakpoint_on_every_instruction: false
        })
    }

}