use log::*;
use crate::{win32::ThreadInfo, *};

pub fn handle_create_thread(debug_context: &DebugContext, mut context: DebuggerContext, event: &CreateThreadEvent) -> Result<Option<DebugEvent>, DebugError> {
    info!("Create thread {}", event);
    
    let thread = ThreadInfo {
        thread_id: event.thread_id,
        // handle: event.handle,
        start_address: event.start_address.0,
        teb: event.teb,
    };
    
    context.insert_thread(thread);

    if context.options.breakpoint_on_every_instruction  {
        context.set_software_breakpoint(event.start_address);
    }
               
    Ok(None)
}

pub fn handle_exit_thread(debug_context: &DebugContext, mut context: DebuggerContext, event: &ExitThreadEvent) -> Result<Option<DebugEvent>, DebugError> {
    
    context.remove_thread(&event.thread_id);
               
    Ok(None)
}