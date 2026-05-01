use log::*;
use crate::{win32::get_handle_flags, *};

pub fn handle_exception(
    debug_context: &DebugContext,
    mut context: DebuggerContext,
    event: &ExceptionEvent,
) -> Result<Option<DebugEvent>, DebugError> {
    // info!("{}", event);
    let breakpoint_on_every_instruction = context.options.breakpoint_on_every_instruction;

    if event.code == ExceptionEventCode::Breakpoint {

        let breakpoint = match context.get_breakpoint(event.address) {
            Some(value) => value,
            None => {
                warn!("Unknown breakpoint at {}", event.address);
                return Ok(None);
                // return Err(DebugError::BreakpointMismatch((event.address)))
            },
        };

        let thread = context.get_thread_mut(debug_context.thread_id)?;

        let code = thread.suspend()?;
        thread.decrease_rip()?;
        thread.resume()?;
        
        let instr = {
            let instr = context.restore_instruction(&breakpoint)?;
        
            if breakpoint_on_every_instruction {
                let next_address = instr.next_address();
                context.set_software_breakpoint(next_address);
            }

            instr
        };

        let kind = BreakpointEvent {
            address: breakpoint.address,
            hit_count: breakpoint.hit_count + 1,
            kind: breakpoint.kind,
            instr,
        };

        let event = DebugEvent {
            kind: DebugEventKind::Breakpoint(kind),
            context: *debug_context,
        };

        return Ok(Some(event))
    }

    Ok(None)
}