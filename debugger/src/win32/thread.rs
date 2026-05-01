use windows_sys::Wdk::System::Threading::*;
use windows_sys::Win32::Foundation::{HANDLE, CloseHandle};
use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::System::Diagnostics::Debug::*;

use crate::{Address, DebugError};

#[repr(C, align(16))]
struct AlignedContext(pub CONTEXT);

#[derive(Clone, Debug)]
pub struct ThreadInfo {
    pub thread_id: u32,
    pub start_address: usize,
    pub teb: Address,
}

#[derive(Clone, Copy, Debug)]
pub enum HardwareBreakpointType {
    Execute,
    Write,
    ReadWrite,
}

#[derive(Clone, Copy, Debug)]
pub enum HardwareBreakpointSize {
    U1,
    U2,
    U4,
    U8,
}

pub fn size_to_len(size: HardwareBreakpointSize) -> u32 {
    match size {
        HardwareBreakpointSize::U1 => 0b00,
        HardwareBreakpointSize::U2 => 0b01,
        HardwareBreakpointSize::U4 => 0b10,
        HardwareBreakpointSize::U8 => 0b11,
    }
}

pub struct Dr7(u64);

impl Dr7 {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn enable_slot(
        &mut self,
        slot: usize,
        kind: HardwareBreakpointType,
        size: HardwareBreakpointSize,
    ) {
        let (rw, len) = match kind {
            HardwareBreakpointType::Execute => (0b00, 0b00),
            HardwareBreakpointType::Write => (0b01, size_to_len(size)),
            HardwareBreakpointType::ReadWrite => (0b11, size_to_len(size)),
        };

        let shift = slot * 2;

        self.0 |= 1 << slot;                 // L0-L3 enable
        self.0 |= (rw as u64) << (16 + shift);
        self.0 |= (len as u64) << (18 + shift);
    }

    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl ThreadInfo {
    fn open_handle(&self) -> Result<HANDLE, DebugError> {
        let flags = THREAD_GET_CONTEXT | THREAD_SET_CONTEXT | THREAD_SUSPEND_RESUME;
        let handle = unsafe {
            OpenThread(
                flags,
                0,
                self.thread_id,
            )
        };
        
        if handle.is_null() {
            return Err(DebugError::ThreadOpenFailed(
                self.thread_id,
                DebugError::last_os_error()
            ));
        }

        Ok(handle)
    }

    pub fn from_id(id: u32) -> Result<Self, DebugError> {
        unsafe {
            let handle = OpenThread(
                THREAD_GET_CONTEXT | THREAD_SET_CONTEXT | THREAD_QUERY_INFORMATION,
                0,
                id,
            );

            if handle.is_null() {
                return Err(DebugError::Other("OpenThread failed".into()));
            }

            let mut ctx: CONTEXT = std::mem::zeroed();
            ctx.ContextFlags = 0x100000;

            if GetThreadContext(handle, &mut ctx) == 0 {
                CloseHandle(handle);
                return Err(DebugError::Other("GetThreadContext failed".into()));
            }

            let teb = {
                let mut teb_ptr: usize = 0;
                let ok = NtQueryInformationThread(
                    handle,
                    9,
                    &mut teb_ptr as *mut _ as *mut _,
                    std::mem::size_of::<usize>() as u32,
                    std::ptr::null_mut(),
                );

                if ok == 0 {
                    teb_ptr
                } else {
                    0
                }
            };

            let ip = ctx.Rip as usize;

            Ok(Self {
                thread_id: id,
                start_address: ip,
                teb: Address(teb),
            })
        }
    }

    pub fn get_rip(&self) -> Result<Address, DebugError> {
        unsafe {
            let handle = self.open_handle()?;

            let mut aligned_ctx = AlignedContext(std::mem::zeroed());
            let ctx = &mut aligned_ctx.0;
            ctx.ContextFlags = CONTEXT_CONTROL_AMD64;
            
            if GetThreadContext(handle, ctx) == 0 {
                return Err(DebugError::GetContextFailed(self.thread_id, DebugError::last_os_error()));
            }
            
            Ok(Address(ctx.Rip as usize))
        }
    }

    pub fn suspend(&self) -> Result<u32, DebugError> {
        let handle = self.open_handle()?;

        let count = unsafe { SuspendThread(handle) };
        if count == u32::MAX {
            let err = DebugError::last_os_error();
            unsafe { CloseHandle(handle) };
            return Err(DebugError::Other(format!("SuspendThread failed: {}", err)));
        }
        
        unsafe { CloseHandle(handle) };

        Ok(count)
    }
    
    pub fn resume(&self) -> Result<u32, DebugError> {
        let handle = self.open_handle()?;

        let count = unsafe { ResumeThread(handle) };
        if count == u32::MAX {
            let err = DebugError::last_os_error();
            unsafe { CloseHandle(handle) };
            return Err(DebugError::Other(format!("ResumeThread failed: {}", err)));
        }
        
        unsafe { CloseHandle(handle) };

        Ok(count)
    }

    pub fn decrease_rip(&self) -> Result<(), DebugError> {
        unsafe {
            let handle = self.open_handle()?;

            let mut aligned_ctx = AlignedContext(std::mem::zeroed());
            let ctx = &mut aligned_ctx.0;
            ctx.ContextFlags = CONTEXT_CONTROL_AMD64;

            if GetThreadContext(handle, ctx) == 0 {
                return Err(DebugError::GetContextFailed(self.thread_id, DebugError::last_os_error()));
            }

            // INT3 is 1 byte → rewind RIP
            ctx.Rip = ctx.Rip.wrapping_sub(1);

            if SetThreadContext(handle, ctx) == 0 {
                return Err(DebugError::SetContextFailed(self.thread_id, DebugError::last_os_error()));
            }

            Ok(())
        }
    }

    // pub fn set_hardware_breakpoint(
    //     &mut self,
    //     address: usize,
    //     slot: usize,
    //     kind: HardwareBreakpointType,
    //     size: HardwareBreakpointSize,
    // ) -> Result<(), DebugError> {
    //     unsafe {
    //         let mut ctx: CONTEXT = std::mem::zeroed();
    //         ctx.ContextFlags = 0x10010; // CONTEXT_DEBUG_REGISTERS

    //         if GetThreadContext(handle, &mut ctx) == 0 {
    //             return Err(DebugError::Other("GetThreadContext failed".into()));
    //         }

    //         match slot {
    //             0 => ctx.Dr0 = address as _,
    //             1 => ctx.Dr1 = address as _,
    //             2 => ctx.Dr2 = address as _,
    //             3 => ctx.Dr3 = address as _,
    //             _ => return Err(DebugError::Other("invalid DR slot".into())),
    //         }

    //         let mut dr7 = Dr7::new();
    //         dr7.enable_slot(slot, kind, size);
    //         ctx.Dr7 = dr7.raw() as u64;

    //         if SetThreadContext(self.handle, &ctx) == 0 {
    //             return Err(DebugError::Other("SetThreadContext failed".into()));
    //         }

    //         Ok(())
    //     }
    // }

    pub fn enable_single_step(&self) -> Result<(), DebugError> {
        unsafe {
            let thread = OpenThread(
                THREAD_GET_CONTEXT | THREAD_SET_CONTEXT,
                0,
                self.thread_id,
            );

            if thread.is_null() {
                return Err(DebugError::ThreadOpenFailed(self.thread_id, DebugError::last_os_error()));
            }

            let mut context: CONTEXT = std::mem::zeroed();
            context.ContextFlags = 0x100000; // CONTEXT_CONTROL

            if GetThreadContext(thread, &mut context) == 0 {
                CloseHandle(thread);
                return Err(DebugError::GetContextFailed(self.thread_id, DebugError::last_os_error()));
            }

            context.EFlags |= 0x100; // Trap Flag

            if SetThreadContext(thread, &context) == 0 {
                CloseHandle(thread);
                return Err(DebugError::SetContextFailed(self.thread_id, DebugError::last_os_error()));
            }

            CloseHandle(thread);
            Ok(())
        }
    }
}

pub fn enumerate_threads(process_id: u32) -> Result<Vec<ThreadInfo>, DebugError> {
    let mut threads = Vec::new();
    
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot.is_null() {
            return Err(DebugError::Other("Failed to create thread snapshot".into()));
        }
        
        let mut entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            cntUsage: 0,
            th32ThreadID: 0,
            th32OwnerProcessID: 0,
            tpBasePri: 0,
            tpDeltaPri: 0,
            dwFlags: 0,
        };
        
        if Thread32First(snapshot, &mut entry) != 0 {
            loop {
                if entry.th32OwnerProcessID == process_id {
                    let thread_info = ThreadInfo::from_id(entry.th32ThreadID)?;
                    threads.push(thread_info);
                }
                
                if Thread32Next(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        
        CloseHandle(snapshot);
    }
    
    Ok(threads)
}