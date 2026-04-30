use core::ptr::null_mut;
use std::collections::HashMap;
use std::sync::Arc;

use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Diagnostics::Debug::*;
use windows_sys::Win32::System::Memory::*;
use windows_sys::Win32::System::Threading::*;

use crate::*;

#[derive(Clone)]
pub struct Breakpoint {
    pub address: Address,
    pub original: [u8; 8],
    pub size: u8,
    pub kind: BreakpointKind,
    pub callback: Option<BreakpointCallback>,
    pub enabled: bool,
}

#[derive(Clone, Copy)]
pub enum BreakpointKind {
    Int3,
    Ud2,
    LongInt3,
}

pub type BreakpointCallback = Arc<dyn Fn() + Send + Sync>;

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
        addr: Address,
        kind: BreakpointKind,
        callback: BreakpointCallback,
        process: HANDLE,
    ) -> Result<(), DebugError> {

        let patch: &[u8] = match kind {
            BreakpointKind::Int3 => &[0xCC],
            BreakpointKind::Ud2 => &[0x0F, 0x0B],
            BreakpointKind::LongInt3 => &[0xCC, 0xCC],
        };
        let size = patch.len() as u8;

        let mut original = [0u8; 8];

        unsafe {
            let mut old = 0;

            VirtualProtectEx(
                process,
                addr.raw() as _,
                size as usize,
                PAGE_EXECUTE_READWRITE,
                &mut old,
            );

            ReadProcessMemory(
                process,
                addr.raw() as _,
                original.as_mut_ptr() as _,
                size as usize,
                null_mut(),
            );

            WriteProcessMemory(
                process,
                addr.raw() as _,
                patch.as_ptr() as _,
                size as usize,
                null_mut(),
            );

            FlushInstructionCache(process, null_mut(), 0);

            VirtualProtectEx(
                process,
                addr.raw() as _,
                size as usize,
                old,
                &mut old,
            );
        }

        self.0.insert(addr, Breakpoint {
            address: addr,
            original,
            size,
            kind,
            callback: Some(callback),
            enabled: true,
        });

        Ok(())
    }

    pub fn remove(
        &mut self,
        addr: Address,
        process: HANDLE,
    ) -> Result<(), DebugError> {
        let breakpoint = self.0.get(&addr).ok_or(DebugError::InvalidState("Breakpoint not found"))?;

        unsafe {
            let mut old = 0;

            VirtualProtectEx(
                process,
                addr.raw() as _,
                breakpoint.size as usize,
                PAGE_EXECUTE_READWRITE,
                &mut old,
            );

            WriteProcessMemory(
                process,
                addr.raw() as _,
                breakpoint.original.as_ptr() as _,
                breakpoint.size as usize,
                null_mut(),
            );

            FlushInstructionCache(process, null_mut(), 0);

            VirtualProtectEx(
                process,
                addr.raw() as _,
                breakpoint.size as usize,
                old,
                &mut old,
            );
        }

        self.0.remove(&addr);

        Ok(())
    }
}