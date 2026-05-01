use windows_sys::Win32::Foundation::{HANDLE, GetLastError};
use windows_sys::Win32::System::Diagnostics::Debug::{FlushInstructionCache, ReadProcessMemory, WriteProcessMemory};
use windows_sys::Win32::System::Memory::{VirtualProtectEx, VirtualQueryEx, MEMORY_BASIC_INFORMATION, PAGE_PROTECTION_FLAGS};

use crate::debugger::DebugError;
use crate::event::Address;

pub fn read_process_memory(
    handle: HANDLE,
    address: Address,
    buffer: &mut [u8],
) -> Result<usize, DebugError> {
    if handle.is_null() {
        return Err(DebugError::InvalidState("process handle is null"));
    }

    if buffer.is_empty() {
        return Ok(0);
    }

    let mut bytes_read = 0;
    let ok = unsafe {
        ReadProcessMemory(
            handle,
            address.0 as *const _,
            buffer.as_mut_ptr() as *mut _,
            buffer.len(),
            &mut bytes_read,
        )
    };

    if ok == 0 {
        let error = unsafe { GetLastError() };
        return Err(DebugError::ReadProcessMemoryFailed(error));
    }

    Ok(bytes_read)
}

pub fn write_process_memory(
    handle: HANDLE,
    address: Address,
    buffer: &[u8],
) -> Result<usize, DebugError> {
    if handle.is_null() {
        return Err(DebugError::InvalidState("process handle is null"));
    }

    if buffer.is_empty() {
        return Ok(0);
    }

    let mut bytes_written = 0;
    let ok = unsafe {
        WriteProcessMemory(
            handle,
            address.0 as *mut _,
            buffer.as_ptr() as *const _,
            buffer.len(),
            &mut bytes_written,
        )
    };

    if ok == 0 {
        let error = unsafe { GetLastError() };
        return Err(DebugError::WriteProcessMemoryFailed(error));
    }

    Ok(bytes_written)
}

pub fn virtual_protect_ex(
    handle: HANDLE,
    address: Address,
    size: usize,
    protect: u32,
) -> Result<u32, DebugError> {
    if handle.is_null() {
        return Err(DebugError::InvalidState("process handle is null"));
    }

    let mut old_protect = 0;
    let ok = unsafe {
        VirtualProtectEx(
            handle,
            address.0 as *mut _,
            size,
            protect,
            &mut old_protect,
        )
    };

    if ok == 0 {
        let error = unsafe { GetLastError() };
        return Err(DebugError::Other(format!("VirtualProtectEx failed: {}", error)));
    }

    Ok(old_protect)
}

pub fn flush_instruction_cache(handle: HANDLE, address: Address, size: usize) -> Result<(), DebugError> {
    unsafe {
        FlushInstructionCache(handle, address.0 as *const _, size);
    }
    Ok(())
}
