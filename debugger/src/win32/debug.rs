use std::ptr::null_mut;

use windows_sys::{Win32::{
    Foundation::{GetHandleInformation, GetLastError, HANDLE, WAIT_TIMEOUT},
    System::{Diagnostics::Debug::*, Threading::OpenProcess},
}, core::BOOL};

use crate::{debugger::DebugError, win32::ProcessHandle};

pub struct ProcessAccessFlags(u32);

impl ProcessAccessFlags {
    pub const EMPTY: Self = Self(0);

    pub const QUERY_INFORMATION: Self = Self(0x0400);
    pub const VM_READ: Self = Self(0x0010);
    pub const VM_WRITE: Self = Self(0x0020);
    pub const VM_OPERATION: Self = Self(0x0008);
    pub const CREATE_THREAD: Self = Self(0x0002);

    pub const ALL_ACCESS: Self = Self(0x001F0FFF);

    pub fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

pub fn get_handle_flags(handle: HANDLE) -> Result<u32, DebugError> {
    let mut flags = 0;
    let result = unsafe { GetHandleInformation(handle, &mut flags) };
    if result == 0 {
        return Err(DebugError::GetHandleFlagsFailed(DebugError::last_os_error()));
    }
    Ok(flags)
}

pub fn open_process(pid: u32, flags: ProcessAccessFlags) -> Result<ProcessHandle, DebugError> {
    let handle = unsafe { OpenProcess(flags.0, 0, pid) };
    
    if handle.is_null() {
        return Err(DebugError::DebugAttachFailed(DebugError::last_os_error()));
    }

     Ok(ProcessHandle {
        pid,
        handle,
        pi: None,
    })
}

pub fn debug_active_process(pid: u32) -> Result<(), DebugError> {
    unsafe {
        let ok: BOOL = DebugActiveProcess(pid);

        if ok == 0 {
            return Err(DebugError::DebugAttachFailed(DebugError::last_os_error()));
        }

        Ok(())
    }
}

pub fn debug_active_process_stop(pid: u32) -> Result<(), DebugError> {
    unsafe {
        let ok: BOOL = DebugActiveProcessStop(pid);

        if ok == 0 {
            return Err(DebugError::DebugDetachFailed(DebugError::last_os_error()));
        }

        Ok(())
    }
}

pub fn wait_event(timeout_ms: u32) -> Result<DEBUG_EVENT, DebugError> {
    unsafe {
        let mut event: DEBUG_EVENT = std::mem::zeroed();
        let ok = WaitForDebugEvent(&mut event, timeout_ms);

        if ok == 0 {
            let err = GetLastError();

            if err == WAIT_TIMEOUT {
                return Err(DebugError::WaitTimeout);
            }

            return Err(DebugError::WaitForDebugEventFailed(err));
        }

        Ok(event)
    }
}

pub fn continue_event(
    pid: u32,
    tid: u32,
    status: i32,
) -> Result<(), DebugError> {
    unsafe {
        let ok: BOOL = ContinueDebugEvent(pid, tid, status);

        if ok == 0 {
            return Err(DebugError::ContinueFailed(DebugError::last_os_error()));
        }

        Ok(())
    }
}