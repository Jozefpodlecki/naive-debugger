use windows_sys::Win32::Foundation::{DBG_CONTINUE, DBG_EXCEPTION_NOT_HANDLED};


pub enum ContinueStatus {
    Continue,
    NotHandled,
}

impl ContinueStatus {
    pub fn as_win32(self) -> i32 {
        match self {
            ContinueStatus::Continue => DBG_CONTINUE,
            ContinueStatus::NotHandled => DBG_EXCEPTION_NOT_HANDLED,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DebugContext {
    pub process_id: u32,
    pub thread_id: u32,
}