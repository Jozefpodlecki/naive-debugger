use std::fmt;

use windows_sys::Win32::{Foundation::HANDLE, System::Diagnostics::Debug::DEBUG_EVENT};

use crate::DebugContext;

pub struct DebugEvent {
    pub kind: DebugEventKind,
    pub context: DebugContext
}

#[derive(Debug, Default)]
pub enum DebugEventKind {
    #[default]
    Idle,
    Exception(ExceptionEvent),
    CreateProcess(CreateProcessEvent),
    ExitProcess(ExitProcessEvent),
    CreateThread(CreateThreadEvent),
    ExitThread(ExitThreadEvent),
    LoadDll(LoadDllEvent),
    UnloadDll(UnloadDllEvent),
    OutputDebugString(OutputDebugStringEvent),
    Rip(RipEvent),
    Unknown(u32),
}

#[derive(Clone, Default, Copy, PartialEq, Eq, Hash)]
pub struct Address(pub usize);

impl Address {
    pub fn new(v: usize) -> Self {
        Self(v)
    }

    pub fn raw(self) -> usize {
        self.0
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:X}", self.0)
    }
}

#[derive(Debug)]
pub struct ExceptionEvent {
    pub code: u32,
    pub address: Address,
    pub first_chance: bool,
}

#[derive(Debug)]
pub struct CreateProcessEvent {
    pub image_base: Address,
    pub entry_point: Address,
    pub file_path: Option<String>,
}

#[derive(Debug)]
pub struct ExitProcessEvent {
    pub process_id: u32,
    pub exit_code: u32,
}

#[derive(Debug)]
pub struct CreateThreadEvent {
    pub thread_id: u32,
    pub handle: HANDLE,
    pub start_address: Address,
    pub teb: Address,
}

#[derive(Debug)]
pub struct ExitThreadEvent {
    pub thread_id: u32,
    pub exit_code: u32,
}

#[derive(Debug)]
pub struct LoadDllEvent {
    pub base_address: Address,
    pub path: Option<String>,
}

#[derive(Debug)]
pub struct UnloadDllEvent {
    pub base_address: Address,
}

#[derive(Debug)]
pub struct OutputDebugStringEvent {
    pub message: String,
}

#[derive(Debug)]
pub struct RipEvent {
    pub error: u32,
    pub type_code: u32,
}
