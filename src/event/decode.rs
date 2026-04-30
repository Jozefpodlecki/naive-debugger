use std::{path::PathBuf, ptr::null_mut};
use windows_sys::Win32::{
    Foundation::HANDLE, Storage::FileSystem::GetFinalPathNameByHandleW, System::{Diagnostics::Debug::*, ProcessStatus::GetMappedFileNameW}
};

use crate::{DebugContext, win32::*};

use super::*;

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


                let mut nt_path = read_wide_string(process, path_ptr);

                if nt_path.is_none() && !info.lpBaseOfDll.is_null() {
                    nt_path = get_mapped_file_name(process, info.lpBaseOfDll);
                }

                let win32_path = get_path_from_file_handle(info.hFile).unwrap();
                let dll_name = PathBuf::from(&win32_path).file_name().unwrap().to_string_lossy().to_string();

                DebugEventKind::LoadDll(LoadDllEvent {
                    base_address: Address(info.lpBaseOfDll as usize),
                    nt_path: nt_path.unwrap(),
                    win32_path,
                    dll_name 
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