use log::*;
use crate::{win32::ThreadInfo, *};

pub fn handle_create_thread(debug_context: &DebugContext, mut context: DebuggerContext, event: &CreateThreadEvent) -> Result<Option<DebugEvent>, DebugError> {
    info!("Create thread {}", event);
    
    let thread = ThreadInfo {
        thread_id: event.thread_id,
        start_address: event.start_address.0,
        teb: event.teb,
    };

    if context.options.breakpoint_on_every_instruction  {
        thread.suspend()?;
        let rip = thread.get_rip()?;
        context.set_software_breakpoint(rip);
        thread.resume()?;
        warn!("RIP {rip} START {}", event.start_address);
        // context.set_software_breakpoint(event.start_address);
    }
    
    context.insert_thread(thread);
               
    Ok(None)
}

pub fn handle_exit_thread(debug_context: &DebugContext, mut context: DebuggerContext, event: &ExitThreadEvent) -> Result<Option<DebugEvent>, DebugError> {
    
    context.remove_thread(&event.thread_id);
               
    Ok(None)
}