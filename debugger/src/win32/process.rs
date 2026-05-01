use std::{os::windows::ffi::OsStrExt, path::Path};
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Diagnostics::Debug::*;
use windows_sys::Win32::System::Threading::DEBUG_ONLY_THIS_PROCESS;
use windows_sys::Win32::{
    System::{
        Threading::{CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW},
    },
};
use windows_sys::core::BOOL;

use crate::debugger::DebugError;
use crate::win32::{ProcessAccessFlags, debug_active_process, open_process};

pub struct CreateProcessBuilder {
    path: Option<Vec<u16>>,
    debug_only_this_process: bool,
}

impl CreateProcessBuilder {
    pub fn new() -> Self {
        Self {
            path: None,
            debug_only_this_process: false,
        }
    }

    pub fn with_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        let wide: Vec<u16> = path
            .as_ref()
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();

        self.path = Some(wide);
        self
    }

    pub fn with_debug_only_this_process(mut self) -> Self {
        self.debug_only_this_process = true;
        self
    }

    pub fn attach(pid: u32, flags: ProcessAccessFlags) -> Result<ProcessHandle, DebugError> {
        debug_active_process(pid)?;
        let handle = open_process(pid, flags)?;

        Ok(handle)
    }

    pub fn spawn(self) -> Result<ProcessHandle, DebugError> {
        let path = self.path.ok_or(DebugError::InvalidState("missing path"))?;

        unsafe {
            let mut si: STARTUPINFOW = std::mem::zeroed();
            si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

            let mut pi: PROCESS_INFORMATION = std::mem::zeroed();

            let mut flags: u32 = 0;
            if self.debug_only_this_process {
                flags |= DEBUG_ONLY_THIS_PROCESS;
            }

            let ok: BOOL = CreateProcessW(
                null_mut(),
                path.as_ptr() as *mut u16,
                null_mut(),
                null_mut(),
                0,
                flags,
                null_mut(),
                null_mut(),
                &si,
                &mut pi,
            );

            if ok == 0 {
                return Err(DebugError::CreateProcessFailed(DebugError::last_os_error()));
            }

            Ok(ProcessHandle {
                pid: pi.dwProcessId,
                handle: pi.hProcess,
                pi: Some(pi),
            })
        }
    }
}

pub struct ProcessHandle {
    pub pid: u32,
    pub handle: HANDLE,
    pub pi: Option<PROCESS_INFORMATION>,
}

impl ProcessHandle {
    pub fn pid(&self) -> u32 {
        self.pid
    }
}