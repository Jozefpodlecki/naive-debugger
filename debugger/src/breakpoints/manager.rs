use core::ptr::null_mut;
use std::collections::HashMap;
use std::sync::Arc;

use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Diagnostics::Debug::*;
use windows_sys::Win32::System::Memory::*;
use windows_sys::Win32::System::Threading::*;

use crate::disasm::DebuggerInstruction;
use crate::win32::*;
use crate::*;

#[derive(Clone)]
pub struct Breakpoint {
    pub address: Address,
    pub original_byte: u8,
    pub original_bytes: Vec<u8>,
    pub instruction: DebuggerInstruction,
    pub kind: BreakpointKind,
    pub enabled: bool,
    pub hit_count: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum BreakpointKind {
    Int3,
    Ud2,
    LongInt3,
}

impl BreakpointKind {
    fn patch_bytes(&self) -> &'static [u8] {
        match self {
            BreakpointKind::Int3 => &[0xCC],
            BreakpointKind::Ud2 => &[0x0F, 0x0B],
            BreakpointKind::LongInt3 => &[0xCC, 0xCC],
        }
    }
}

pub struct BreakpointManager(HashMap<Address, Breakpoint>);

impl BreakpointManager {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn contains(&self, addr: Address) -> bool {
        self.0.contains_key(&addr)
    }

    pub fn get(&self, addr: Address) -> Option<&Breakpoint> {
        self.0.get(&addr)
    }

        pub fn set(
        &mut self,
        address: Address,
        kind: BreakpointKind,
        process_handle: HANDLE,
    ) -> Result<(), DebugError> {
        let patch = kind.patch_bytes();
        
        let mut full_buffer = vec![0u8; 15];
        let bytes_read = read_process_memory(process_handle, address, &mut full_buffer)?;
        
        let mut decoder = iced_x86::Decoder::with_ip(
            64, 
            &full_buffer[..bytes_read], 
            address.0 as u64, 
            iced_x86::DecoderOptions::NONE
        );
        
        let instruction = decoder.decode();
        
        if instruction.is_invalid() {
            return Err(DebugError::InvalidInstruction(decoder.last_error()));
        }
        
        let instr_len = instruction.len() as usize;
        let original_bytes = full_buffer[..instr_len].to_vec();
        let original_byte = original_bytes[0];
        
        let old_protect = virtual_protect_ex(process_handle, address, patch.len(), PAGE_EXECUTE_READWRITE)?;
        write_process_memory(process_handle, address, patch)?; 
        flush_instruction_cache(process_handle, address, 1)?;
        virtual_protect_ex(process_handle, address, 1, old_protect)?;
        
        self.0.insert(address, Breakpoint {
            address,
            original_byte,
            original_bytes,
            instruction: DebuggerInstruction(instruction),
            kind,
            enabled: true,
            hit_count: 0,
        });
        
        Ok(())
    }

    pub fn remove(
        &mut self,
        addr: Address,
        process_handle: HANDLE,
    ) -> Result<(), DebugError> {
        let breakpoint = self.0.get(&addr)
            .ok_or(DebugError::BreakpointNotFound(addr))?;

        let size = breakpoint.original_bytes.len();

        let old_protect = virtual_protect_ex(
            process_handle,
            addr,
            size,
            PAGE_EXECUTE_READWRITE,
        )?;

        let bytes_written = write_process_memory(
            process_handle,
            addr,
            &breakpoint.original_bytes,
        )?;

        if bytes_written != size {
            return Err(DebugError::Other(format!(
                "Failed to restore original bytes: expected {} bytes, got {}",
                size, bytes_written
            )));
        }

        flush_instruction_cache(process_handle, addr, size)?;
        virtual_protect_ex(process_handle, addr, size, old_protect)?;

        self.0.remove(&addr);

        Ok(())
    }

    pub fn disable(&mut self, addr: Address, process_handle: HANDLE) -> Result<(), DebugError> {
        if let Some(breakpoint) = self.0.get_mut(&addr) {
            if breakpoint.enabled {
                let size = breakpoint.original_bytes.len();
                
                let old_protect = virtual_protect_ex(process_handle, addr, size, PAGE_EXECUTE_READWRITE)?;
                write_process_memory(process_handle, addr, &breakpoint.original_bytes)?;
                flush_instruction_cache(process_handle, addr, size)?;
                virtual_protect_ex(process_handle, addr, size, old_protect)?;
                
                breakpoint.enabled = false;
            }
        }
        Ok(())
    }

    pub fn enable(&mut self, addr: Address, process_handle: HANDLE) -> Result<(), DebugError> {
        if let Some(breakpoint) = self.0.get_mut(&addr) {
            if !breakpoint.enabled {
                let patch = breakpoint.kind.patch_bytes();
                let size = patch.len();
                
                let old_protect = virtual_protect_ex(process_handle, addr, size, PAGE_EXECUTE_READWRITE)?;
                write_process_memory(process_handle, addr, patch)?;
                flush_instruction_cache(process_handle, addr, size)?;
                virtual_protect_ex(process_handle, addr, size, old_protect)?;
                
                breakpoint.enabled = true;
            }
        }
        Ok(())
    }
}