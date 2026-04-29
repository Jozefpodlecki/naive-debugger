use core::ptr::null_mut;
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

pub struct BreakpointManager {
    list: Vec<Breakpoint>,
}

impl BreakpointManager {
     pub fn set(
        &mut self,
        addr: Address,
        kind: BreakpointKind,
        callback: BreakpointCallback,
        process: HANDLE,
    ) -> Result<(), DebugError> {
        if self.list.iter().any(|b| b.address == addr && b.enabled) {
            return Ok(());
        }

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

        self.list.push(Breakpoint {
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
        let idx = self
            .list
            .iter()
            .position(|b| b.address == addr)
            .ok_or(DebugError::InvalidState("BPX not found"))?;

        let bp = &self.list[idx];

        unsafe {
            let mut old = 0;

            VirtualProtectEx(
                process,
                addr.raw() as _,
                bp.size as usize,
                PAGE_EXECUTE_READWRITE,
                &mut old,
            );

            WriteProcessMemory(
                process,
                addr.raw() as _,
                bp.original.as_ptr() as _,
                bp.size as usize,
                null_mut(),
            );

            FlushInstructionCache(process, null_mut(), 0);

            VirtualProtectEx(
                process,
                addr.raw() as _,
                bp.size as usize,
                old,
                &mut old,
            );
        }

        self.list.remove(idx);

        Ok(())
    }
}