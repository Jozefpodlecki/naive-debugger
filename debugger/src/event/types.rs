use std::{fmt, ops::Add};

use iced_x86::Instruction;
use windows_sys::Win32::{Foundation::{HANDLE, STATUS_BREAKPOINT}, System::Diagnostics::Debug::DEBUG_EVENT};

use crate::{DebugContext, breakpoints::BreakpointKind, disasm::DebuggerInstruction};

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
    Breakpoint(BreakpointEvent),  
    Unknown(u32),
}

impl fmt::Display for DebugEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.context, self.kind)
    }
}

impl fmt::Display for DebugContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PID:{} TID:{}", self.process_id, self.thread_id)
    }
}

impl fmt::Display for DebugEventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DebugEventKind::Idle => write!(f, "Idle"),
            DebugEventKind::Exception(e) => write!(f, "Exception: {}", e),
            DebugEventKind::CreateProcess(e) => write!(f, "CreateProcess: {}", e),
            DebugEventKind::ExitProcess(e) => write!(f, "ExitProcess: {}", e),
            DebugEventKind::CreateThread(e) => write!(f, "CreateThread: {}", e),
            DebugEventKind::ExitThread(e) => write!(f, "ExitThread: {}", e),
            DebugEventKind::LoadDll(e) => write!(f, "LoadDll: {}", e),
            DebugEventKind::UnloadDll(e) => write!(f, "UnloadDll: {}", e),
            DebugEventKind::OutputDebugString(e) => write!(f, "OutputDebugString: {}", e),
            DebugEventKind::Rip(e) => write!(f, "Rip: {}", e),
            DebugEventKind::Breakpoint(event) => write!(f, "Breakpoint: {}", event),
            DebugEventKind::Unknown(code) => write!(f, "Unknown event (0x{:08X})", code),
        }
    }
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

impl Add<usize> for Address {
    type Output = Address;
    
    fn add(self, rhs: usize) -> Address {
        Address(self.0 + rhs)
    }
}

impl Add for Address {
    type Output = Address;
    
    fn add(self, rhs: Self) -> Address {
        Address(self.0 + rhs.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionEventCode {
    Breakpoint,           // INT3 instruction (0x80000003)
    SingleStep,           // Trace flag exception (0x80000004)
    
    // Memory exceptions
    AccessViolation,      // Read/write/execute invalid memory (0xC0000005)
    GuardPageViolation,   // Guard page hit (0x80000001)
    DatatypeMisalignment, // Unaligned memory access (0x80000002)
    
    // Instruction exceptions
    IllegalInstruction,   // Invalid opcode, UD2 (0xC000001D)
    PrivilegedInstruction,// Ring 0 instruction in user mode (0xC0000096)
    InvalidLockSequence,  // Invalid LOCK prefix usage (0x80000019)
    
    // Arithmetic exceptions
    IntegerDivisionByZero,   // (0xC0000094)
    IntegerOverflow,         // (0xC0000095)
    FloatDivideByZero,       // (0xC000008E)
    FloatOverflow,           // (0xC0000091)
    FloatUnderflow,          // (0xC0000093)
    FloatInexactResult,      // (0xC0000092)
    
    // Stack exceptions
    StackOverflow,        // (0xC00000FD)
    
    // Process exceptions
    ControlCExit,         // Ctrl+C in console (0x40010005)
    
    // Unknown
    Unknown(u32),
}

impl fmt::Display for ExceptionEventCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ExceptionEventCode::Breakpoint => "STATUS_BREAKPOINT",
            ExceptionEventCode::SingleStep => "STATUS_SINGLE_STEP",
            ExceptionEventCode::AccessViolation => "STATUS_ACCESS_VIOLATION",
            ExceptionEventCode::GuardPageViolation => "STATUS_GUARD_PAGE_VIOLATION",
            ExceptionEventCode::DatatypeMisalignment => "STATUS_DATATYPE_MISALIGNMENT",
            ExceptionEventCode::IllegalInstruction => "STATUS_ILLEGAL_INSTRUCTION",
            ExceptionEventCode::PrivilegedInstruction => "STATUS_PRIVILEGED_INSTRUCTION",
            ExceptionEventCode::InvalidLockSequence => "STATUS_INVALID_LOCK_SEQUENCE",
            ExceptionEventCode::IntegerDivisionByZero => "STATUS_INTEGER_DIVIDE_BY_ZERO",
            ExceptionEventCode::IntegerOverflow => "STATUS_INTEGER_OVERFLOW",
            ExceptionEventCode::FloatDivideByZero => "STATUS_FLOAT_DIVIDE_BY_ZERO",
            ExceptionEventCode::FloatOverflow => "STATUS_FLOAT_OVERFLOW",
            ExceptionEventCode::FloatUnderflow => "STATUS_FLOAT_UNDERFLOW",
            ExceptionEventCode::FloatInexactResult => "STATUS_FLOAT_INEXACT_RESULT",
            ExceptionEventCode::StackOverflow => "STATUS_STACK_OVERFLOW",
            ExceptionEventCode::ControlCExit => "STATUS_CONTROL_C_EXIT",
            ExceptionEventCode::Unknown(code) => return write!(f, "UNKNOWN_0x{:08X}", code),
        };
        write!(f, "{}", s)
    }
}


#[allow(non_snake_case)]
impl From<i32> for ExceptionEventCode {
    fn from(code: i32) -> Self {
        match code {
            STATUS_BREAKPOINT => ExceptionEventCode::Breakpoint,
            STATUS_SINGLE_STEP => ExceptionEventCode::SingleStep,
            STATUS_ACCESS_VIOLATION => ExceptionEventCode::AccessViolation,
            STATUS_GUARD_PAGE_VIOLATION => ExceptionEventCode::GuardPageViolation,
            STATUS_DATATYPE_MISALIGNMENT => ExceptionEventCode::DatatypeMisalignment,
            STATUS_ILLEGAL_INSTRUCTION => ExceptionEventCode::IllegalInstruction,
            STATUS_PRIVILEGED_INSTRUCTION => ExceptionEventCode::PrivilegedInstruction,
            STATUS_INVALID_LOCK_SEQUENCE => ExceptionEventCode::InvalidLockSequence,
            STATUS_INTEGER_DIVIDE_BY_ZERO => ExceptionEventCode::IntegerDivisionByZero,
            STATUS_INTEGER_OVERFLOW => ExceptionEventCode::IntegerOverflow,
            STATUS_FLOAT_DIVIDE_BY_ZERO => ExceptionEventCode::FloatDivideByZero,
            STATUS_FLOAT_OVERFLOW => ExceptionEventCode::FloatOverflow,
            STATUS_FLOAT_UNDERFLOW => ExceptionEventCode::FloatUnderflow,
            STATUS_FLOAT_INEXACT_RESULT => ExceptionEventCode::FloatInexactResult,
            STATUS_STACK_OVERFLOW => ExceptionEventCode::StackOverflow,
            STATUS_CONTROL_C_EXIT => ExceptionEventCode::ControlCExit,
            _ => ExceptionEventCode::Unknown(code as u32),
        }
    }
}

#[derive(Debug)]
pub struct ExceptionEvent {
    pub code: ExceptionEventCode,
    pub address: Address,
    pub first_chance: bool,
}

impl ExceptionEvent {
    pub fn is_debug_exception(&self) -> bool {
        matches!(self.code, 
            ExceptionEventCode::Breakpoint | 
            ExceptionEventCode::SingleStep
        )
    }
    
    pub fn is_fatal(&self) -> bool {
        matches!(self.code,
            ExceptionEventCode::AccessViolation |
            ExceptionEventCode::IllegalInstruction |
            ExceptionEventCode::StackOverflow |
            ExceptionEventCode::IntegerDivisionByZero |
            ExceptionEventCode::FloatDivideByZero
        )
    }
}

#[derive(Debug)]
pub struct CreateProcessEvent {
    pub image_base: Address,
    pub entry_point: Address,
    pub file_path: String,
    pub main_thread_handle: HANDLE,
    pub teb: Address,
    pub process_handle: HANDLE,
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
    pub dll_name: String,
    pub nt_path: String,
    pub win32_path: String,
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

pub struct BreakpointEvent {
    pub address: Address,
    pub hit_count: u32,
    pub kind: BreakpointKind,
    pub instr: DebuggerInstruction
}

impl fmt::Display for BreakpointEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, "{} (hit_count={}, kind={:?}) - {}",
            self.address, self.hit_count, self.kind, self.instr
        )
    }
}


impl fmt::Display for ExceptionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let chance = if self.first_chance { "first" } else { "second" };
        write!(f, "{} at {} ({}-chance)", self.code, self.address, chance)
    }
}

impl fmt::Display for CreateProcessEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} entry at {}", self.file_path, self.entry_point)
    }
}

impl fmt::Display for ExitProcessEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PID={} exit_code=0x{:08X}", self.process_id, self.exit_code)
    }
}

impl fmt::Display for CreateThreadEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TID={} start={}", self.thread_id, self.start_address)
    }
}

impl fmt::Display for ExitThreadEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TID={} exit_code=0x{:08X}", self.thread_id, self.exit_code)
    }
}

impl fmt::Display for LoadDllEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {} ({})", self.dll_name, self.base_address, self.win32_path)
    }
}

impl fmt::Display for UnloadDllEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DLL at {}", self.base_address)
    }
}

impl fmt::Display for OutputDebugStringEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.message)
    }
}

impl fmt::Display for RipEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error=0x{:08X} type=0x{:08X}", self.error, self.type_code)
    }
}

impl fmt::Debug for BreakpointEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, "BreakpointEvent {{ address: {}, hit_count: {}, kind: {:?}, instr: {} }}",
            self.address, self.hit_count, self.kind, self.instr
        )
    }
}