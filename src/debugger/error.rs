use std::{fmt, io};
use windows_sys::Win32::Foundation::GetLastError;

#[derive(Debug)]
pub enum DebugError {
    CreateProcessFailed(u32),
    WaitForDebugEventFailed(u32),
    ContinueDebugEventFailed(u32),
    ReadProcessMemoryFailed(u32),
    DebugAttachFailed(u32),
    DebugDetachFailed(u32),
    ContinueFailed(u32),
    ThreadOpenFailed(u32),
    GetContextFailed(u32),
    SetContextFailed(u32),
    InvalidState(&'static str),
    WaitTimeout,
    Io(io::Error),
    Other(String),
}

impl fmt::Display for DebugError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DebugError::CreateProcessFailed(err) =>
                write!(f, "CreateProcessW failed (error={})", err),

            DebugError::WaitForDebugEventFailed(err) =>
                write!(f, "WaitForDebugEvent failed (error={})", err),

            DebugError::ContinueDebugEventFailed(err) =>
                write!(f, "ContinueDebugEvent failed (error={})", err),

            DebugError::ReadProcessMemoryFailed(err) =>
                write!(f, "ReadProcessMemory failed (error={})", err),

            DebugError::DebugAttachFailed(err) =>
                write!(f, "DebugActiveProcess failed (error={})", err),

            DebugError::DebugDetachFailed(err) =>
                write!(f, "DebugActiveProcessStop failed (error={})", err),

            DebugError::InvalidState(s) =>
                write!(f, "Invalid debugger state: {}", s),

            DebugError::WaitTimeout =>
                write!(f, "WaitForDebugEvent timed out"),

            DebugError::Io(err) =>
                write!(f, "IO error: {}", err),

            DebugError::ContinueFailed(err) =>
                write!(f, "ContinueDebugEvent failed (error={})", err),

            DebugError::Other(s) =>
                write!(f, "{}", s),
                            DebugError::ThreadOpenFailed(_) => todo!(),
                            DebugError::GetContextFailed(_) => todo!(),
                            DebugError::SetContextFailed(_) => todo!(),
        }
    }
}

impl std::error::Error for DebugError {}

impl DebugError {
    pub fn last_os_error() -> u32 {
        unsafe { GetLastError() }
    }
}