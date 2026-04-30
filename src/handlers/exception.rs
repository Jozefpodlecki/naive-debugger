use iced_x86::Instruction;

use crate::*;

pub struct BreakpointEvent {
    instr: Instruction
}

pub fn handle_exception(
    context: DebuggerContext,
    event: &ExceptionEvent,
) -> Result<BreakpointEvent, DebugError> {
    
    if event.is_status_breakpoint() {

        let breakpoint = match context.get_breakpoint(event.address) {
            Some(value) => value,
            None => return Err(DebugError::BreakpointMismatch((event.address))),
        };

        context.remove_breakpoint(event.address);

        context.decrease_rip()?;
        let instr = context.restore_instruction(breakpoint)?;
        let next_address = context.next_address(instr);
        
        if context.breakpoint_on_every_instruction {
            context.set_software_breakpoint(next_address);
        }
    }

    Ok(BreakpointEvent {

    })
}