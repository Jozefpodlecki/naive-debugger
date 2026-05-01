use std::{fs::File, io::Read};

use pelite::pe::{Pe, PeFile};

use crate::{modules::Module, win32::ThreadInfo, *};


pub fn handle_create_process(
    debug_context: &DebugContext,
    mut context: DebuggerContext,
    event: &CreateProcessEvent) -> Result<Option<DebugEvent>, DebugError> {
    
    let path = &event.file_path;
    let mut file = File::open(path).map_err(|e| DebugError::Other(format!("Failed to open main exe: {}", e)))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| DebugError::Other(format!("Failed to read main exe: {}", e)))?;
    
    let pe = PeFile::from_bytes(&buffer).map_err(|e| DebugError::Other(format!("Invalid PE: {:?}", e)))?;
    let optional_header = pe.optional_header();
    let entry_point_rva = optional_header.AddressOfEntryPoint as usize;
    let size_of_image = optional_header.SizeOfImage as usize;
    
    let entry_point = if entry_point_rva != 0 {
        Some(Address(event.image_base.0 + entry_point_rva))
    } else {
        None
    };
    
    let name = std::path::Path::new(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    
    let module = Module {
        path: path.clone(),
        base_address: event.image_base,
        end_address: Address(event.image_base.0 + size_of_image),
        entry_point,
        name,
        size_of_image,
    };
    
    context.insert_module(module);

    let thread_info = ThreadInfo {
        thread_id: debug_context.thread_id,
        start_address: event.entry_point.0,
        teb: event.teb,
    };
    
    context.insert_thread(thread_info);
    
    if context.options.breakpoint_on_every_instruction {
        context.set_software_breakpoint(event.entry_point)?;
    }
    
    Ok(None)
}