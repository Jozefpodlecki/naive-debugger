use std::{fs::File, io::Read};

use pelite::pe::{Pe, PeFile};

use crate::{modules::Module, *};


pub fn handle_load_dll(
    debug_context: &DebugContext,
    mut context: DebuggerContext,
    event: &LoadDllEvent) -> Result<Option<DebugEvent>, DebugError> {
    
    let mut file = File::open(&event.win32_path).map_err(|e| DebugError::Other(format!("Failed to open {}: {}", event.win32_path, e)))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| DebugError::Other(format!("Failed to read {}: {}", event.win32_path, e)))?;

    let pe = PeFile::from_bytes(&buffer).map_err(|e| DebugError::Other(format!("Invalid PE: {:?}", e)))?;
    let entry_point_rva = pe.optional_header().AddressOfEntryPoint as usize;
    
    let entry_point = if entry_point_rva != 0 {
        Some(Address(event.base_address.0 + entry_point_rva))
    } else {
        None
    };

    let size_of_image = pe.optional_header().SizeOfImage as usize;

    let module = Module {
        path: event.win32_path.clone(),
        base_address: event.base_address,
        end_address: Address(event.base_address.0 + size_of_image), 
        entry_point: entry_point,
        name: event.dll_name.clone(),
        size_of_image,
    };

    if let Some(addr) = module.entry_point {
        context.set_software_breakpoint(addr)?;
    }
    else {
        println!("Could not set breakpoint for DLL {}", module.name);
    }

    context.insert_module(module);
       
    Ok(None)
}

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
