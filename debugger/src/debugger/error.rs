use std::{fmt, io, path::PathBuf};
use iced_x86::DecoderError;
use windows_sys::Win32::Foundation::GetLastError;

use crate::Address;

#[derive(Debug)]
pub enum DebugError {
    BinaryNotFound(PathBuf),
    CreateProcessFailed(u32),
    WaitForDebugEventFailed(u32),
    ContinueDebugEventFailed(u32),
    ReadProcessMemoryFailed(u32),
    WriteProcessMemoryFailed(u32),
    DebugAttachFailed(u32),
    DebugDetachFailed(u32),
    ContinueFailed(u32),
    ThreadOpenFailed(u32, u32),
    GetContextFailed(u32, u32),
    SetContextFailed(u32, u32),
    BreakpointMismatch(Address),
    BreakpointNotFound(Address),
    InvalidState(&'static str),
    WaitTimeout,
    InvalidInstruction(DecoderError),
    ThreadNotFound(u32),
    GetHandleFlagsFailed(u32),
    Io(io::Error),
    Other(String),
}

impl fmt::Display for DebugError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DebugError::BinaryNotFound(path) =>
                write!(f, "Could not find binary at path {}", path.display()),

            DebugError::CreateProcessFailed(err) =>
                write!(f, "CreateProcessW failed (error={})", err),

            DebugError::WaitForDebugEventFailed(err) =>
                write!(f, "WaitForDebugEvent failed (error={})", err),

            DebugError::ContinueDebugEventFailed(err) =>
                write!(f, "ContinueDebugEvent failed (error={})", err),

            DebugError::ReadProcessMemoryFailed(err) =>
                write!(f, "ReadProcessMemory failed (error={})", err),

            DebugError::WriteProcessMemoryFailed(err) =>
                write!(f, "WriteProcessMemory failed (error={})", err),

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

            DebugError::ThreadOpenFailed(thread_id, err) =>
                write!(f, "OpenThread failed (thread_id={}, error={})", thread_id, err),

            DebugError::GetContextFailed(thread_id, err) =>
                write!(f, "GetThreadContext failed (thread_id={}, error={})", thread_id, err),

            DebugError::SetContextFailed(thread_id, err) =>
                write!(f, "SetThreadContext failed (thread_id={}, error={})", thread_id, err),

            DebugError::BreakpointMismatch(address) =>
                write!(f, "Breakpoint mismatch at address {}", address),

            DebugError::BreakpointNotFound(address) =>
                write!(f, "Breakpoint not found at address {}", address),

            DebugError::ThreadNotFound(id) =>
                write!(f, "Thread {} not found", id),

            DebugError::InvalidInstruction(error) =>
                write!(f, "Invalid instruction: {:?}", error),

            DebugError::GetHandleFlagsFailed(err) =>
                write!(f, "GetHandleInformation failed (error={})", err),
        }
    }
}

impl std::error::Error for DebugError {}

impl DebugError {
    pub fn last_os_error() -> u32 {
        unsafe { GetLastError() }
    }
}