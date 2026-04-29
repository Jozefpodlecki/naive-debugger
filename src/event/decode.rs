use std::ptr::null_mut;
use windows_sys::Win32::{
    Foundation::HANDLE, Storage::FileSystem::GetFinalPathNameByHandleW, System::Diagnostics::Debug::*
};

use crate::DebugContext;

use super::*;

fn get_path_from_handle(handle: HANDLE) -> Option<String> {
    if handle.is_null() {
        return None;
    }

    let mut buf = vec![0u16; 260];

    let len = unsafe {
        GetFinalPathNameByHandleW(
            handle,
            buf.as_mut_ptr(),
            buf.len() as u32,
            0,
        )
    };

    if len == 0 {
        return None;
    }

    Some(String::from_utf16_lossy(&buf[..len as usize]))
}

pub unsafe fn read_remote<T: Copy>(
    process: HANDLE,
    addr: *const core::ffi::c_void,
) -> Option<T> {
    let mut out: T = std::mem::zeroed();
    let mut bytes = 0;

    let ok = ReadProcessMemory(
        process,
        addr,
        &mut out as *mut _ as *mut _,
        std::mem::size_of::<T>(),
        &mut bytes,
    );

    if ok == 0 {
        None
    } else {
        Some(out)
    }
}

pub fn decode_output_debug_string(
    process: HANDLE,
    info: &OUTPUT_DEBUG_STRING_INFO,
) -> String {
    use std::ptr::null_mut;

    unsafe {
        let mut buffer = vec![0u8; info.nDebugStringLength as usize + 1];

        let mut bytes_read: usize = 0;

        windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory(
            process,
            info.lpDebugStringData as *const _,
            buffer.as_mut_ptr() as *mut _,
            info.nDebugStringLength as usize,
            &mut bytes_read as *mut _,
        );

        // ANSI or Unicode flag decides interpretation
        if info.fUnicode != 0 {
            let wide = std::slice::from_raw_parts(
                buffer.as_ptr() as *const u16,
                (bytes_read / 2).max(1),
            );

            String::from_utf16_lossy(wide)
        } else {
            String::from_utf8_lossy(&buffer[..bytes_read]).to_string()
        }
    }
}

pub fn read_wide_string(process: HANDLE, ptr: *const u16) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    let mut buf = [0u16; 260];
    let mut bytes = 0;

    let ok = unsafe {
        ReadProcessMemory(
            process,
            ptr as *const _,
            buf.as_mut_ptr() as *mut _,
            std::mem::size_of_val(&buf),
            &mut bytes,
        )
    };

    if ok == 0 {
        return None;
    }

    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16(buf[..len].to_vec().as_slice()).ok()
}

impl DebugEvent {
    pub fn decode(event: &DEBUG_EVENT, process: HANDLE) -> Self {
        let context = DebugContext {
        process_id: event.dwProcessId,
        thread_id: event.dwThreadId,
    };

    unsafe {
        let kind = match event.dwDebugEventCode {
            EXCEPTION_DEBUG_EVENT => {
                let info = event.u.Exception;

                DebugEventKind::Exception(ExceptionEvent {
                    code: info.ExceptionRecord.ExceptionCode as u32,
                    address: Address(info.ExceptionRecord.ExceptionAddress as usize),
                    first_chance: info.dwFirstChance != 0,
                })
            }

            CREATE_PROCESS_DEBUG_EVENT => {
                let info = event.u.CreateProcessInfo;

                let file_path = read_wide_string(process, info.lpImageName as *const u16)
                    .or_else(|| get_path_from_handle(info.hFile));

                DebugEventKind::CreateProcess(CreateProcessEvent {
                    image_base: Address(info.lpBaseOfImage as usize),
                    entry_point: info.lpStartAddress
                        .map(|f| Address(f as usize))
                        .unwrap_or_default(),
                    file_path,
                })
            }

            EXIT_PROCESS_DEBUG_EVENT => {
                let info = event.u.ExitProcess;

                DebugEventKind::ExitProcess(ExitProcessEvent {
                    process_id: event.dwProcessId,
                    exit_code: info.dwExitCode,
                })
            }

            CREATE_THREAD_DEBUG_EVENT => {
                let info = event.u.CreateThread;

                DebugEventKind::CreateThread(CreateThreadEvent {
                    thread_id: event.dwThreadId,
                    teb: Address(info.lpThreadLocalBase as usize),
                    handle: info.hThread,
                    start_address: info.lpStartAddress
                        .map(|f| Address(f as usize))
                        .unwrap_or_default(),
                })
            }

            EXIT_THREAD_DEBUG_EVENT => {
                let info = event.u.ExitThread;

                DebugEventKind::ExitThread(ExitThreadEvent {
                    thread_id: event.dwThreadId,
                    exit_code: info.dwExitCode,
                })
            }

            LOAD_DLL_DEBUG_EVENT => {
                let info = event.u.LoadDll;

                let path_ptr: *const u16 =
                    read_remote(process, info.lpImageName as *const _)
                        .unwrap_or(null_mut());

                let path = read_wide_string(process, path_ptr);

                DebugEventKind::LoadDll(LoadDllEvent {
                    base_address: Address(info.lpBaseOfDll as usize),
                    path,
                })
            }

            UNLOAD_DLL_DEBUG_EVENT => {
                let info = event.u.UnloadDll;

                DebugEventKind::UnloadDll(UnloadDllEvent {
                    base_address: Address(info.lpBaseOfDll as usize),
                })
            }

            OUTPUT_DEBUG_STRING_EVENT => {
                let info = event.u.DebugString;

                let message = {
                    let mut buffer = vec![0u8; info.nDebugStringLength as usize + 1];
                    let mut bytes_read: usize = 0;

                    ReadProcessMemory(
                        process,
                        info.lpDebugStringData as *const _,
                        buffer.as_mut_ptr() as *mut _,
                        info.nDebugStringLength as usize,
                        &mut bytes_read as *mut _,
                    );

                    if info.fUnicode != 0 {
                        let wide = std::slice::from_raw_parts(
                            buffer.as_ptr() as *const u16,
                            (bytes_read / 2).max(1),
                        );

                        String::from_utf16_lossy(wide)
                    } else {
                        String::from_utf8_lossy(&buffer[..bytes_read]).to_string()
                    }
                };

                DebugEventKind::OutputDebugString(OutputDebugStringEvent {
                    message,
                })
            }

            RIP_EVENT => {
                let info = event.u.RipInfo;

                DebugEventKind::Rip(RipEvent {
                    error: info.dwError,
                    type_code: info.dwType,
                })
            }

            code => DebugEventKind::Unknown(code),
        };

        DebugEvent { kind, context }
    }
}
}