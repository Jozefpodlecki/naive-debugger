use std::ptr::null_mut;
use pelite::image::{IMAGE_DOS_HEADER, IMAGE_NT_HEADERS64};
use windows_sys::Win32::{
    Foundation::HANDLE, Storage::FileSystem::*, System::{Diagnostics::{Debug::*, ToolHelp::*}, ProcessStatus::GetMappedFileNameW}
};
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::System::ProcessStatus::*;
use windows_sys::Win32::Foundation::CloseHandle;

use crate::{Address, DebugError, read_process_memory};

pub fn get_path_from_file_handle(handle: HANDLE) -> Option<String> {
    if handle.is_null() {
        return None;
    }
    
    let mut buffer = vec![0u16; 260];
    
    unsafe {
        let len = GetFinalPathNameByHandleW(
            handle,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            0x0, // VOLUME_NAME_DOS - gives "C:\..." format
        );
        
        if len == 0 {
            return None;
        }
        
        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        
        // GetFinalPathNameByHandleW returns "\\?\C:\..." format
        // Remove the "\\?\" prefix for normal file operations
        let clean_path = path.trim_start_matches("\\\\?\\").to_string();
        
        Some(clean_path)
    }
}

pub fn get_mapped_file_name(
    process: HANDLE,
    address: *const core::ffi::c_void,
) -> Option<String> {
    if process.is_null() || address.is_null() {
        return None;
    }

    let mut buffer = vec![0u16; 260];
    
    unsafe {
        let len = GetMappedFileNameW(
            process,
            address,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
        );
        
        if len == 0 {
            return None;
        }
        
        let nt_path = String::from_utf16_lossy(&buffer[..len as usize]);
        Some(nt_path_to_dos_path(&nt_path))
        // Some(nt_path)
    }
}

pub fn nt_path_to_dos_path(nt_path: &str) -> String {
    let drives = get_logical_drives();
    
    for drive in drives {
        let device_path = get_device_path_for_drive(&drive);
        if nt_path.starts_with(&device_path) {
            return nt_path.replacen(&device_path, &drive, 1);
        }
    }
    
    nt_path.to_string()
}

// pub fn nt_path_to_dos_path(nt_path: &str) -> String {
//     // Clean up any extra backslashes first
//     let clean_path = nt_path.replace("\\\\", "\\");
    
//     // Known device mappings
//     let device_mappings = unsafe {
//         get_device_to_drive_mappings()
//     };
    
//     for (device, drive) in device_mappings {
//         if clean_path.starts_with(&device) {
//             return clean_path.replacen(&device, &drive, 1);
//         }
//     }
    
//     // If no mapping found, return as-is (might still work or show error)
//     clean_path
// }

fn get_device_path_for_drive(drive: &str) -> String {
    
    let drive_letter = &drive[..2]; // "C:"
    let mut device_path = vec![0u16; 260];
    
    unsafe {
        let len = QueryDosDeviceW(
            drive_letter.as_ptr() as *const u16,
            device_path.as_mut_ptr(),
            device_path.len() as u32,
        );
        
        if len > 0 {
            String::from_utf16_lossy(&device_path[..len as usize])
        } else {
            String::new()
        }
    }
}

pub fn get_logical_drives() -> Vec<String> {
    let mut drives = Vec::new();
    unsafe {
        let drive_mask = GetLogicalDrives();
        for i in 0..26 {
            if drive_mask & (1 << i) != 0 {
                let drive_letter = char::from(b'A' + i as u8);
                drives.push(format!("{}:\\", drive_letter));
            }
        }
    }
    drives
}

pub fn get_path_from_handle(handle: HANDLE) -> Option<String> {
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

pub fn read_remote<T: Copy>(
    process: HANDLE,
    addr: *const core::ffi::c_void,
) -> Option<T> {
    let mut out: T = unsafe { std::mem::zeroed() };
    let mut bytes = 0;

    let ok = unsafe {
        ReadProcessMemory(
            process,
            addr,
            &mut out as *mut _ as *mut _,
            std::mem::size_of::<T>(),
            &mut bytes,
        )
    };

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

        ReadProcessMemory(
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

pub fn find_process_by_name(name: &str) -> Option<u32> {

    unsafe {
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..std::mem::zeroed()
        };
        
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot.is_null() {
            return None;
        }
        
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let len = entry.szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(0);
                
                let process_name = if len > 0 {
                    String::from_utf16_lossy(&entry.szExeFile[..len])
                } else {
                    String::new()
                };

                if process_name.to_lowercase() == name.to_lowercase() {
                    CloseHandle(snapshot);
                    return Some(entry.th32ProcessID);
                }
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        
        CloseHandle(snapshot);
        None
    }
}

pub fn read_remote_value<T: Copy>(process: HANDLE, address: Address) -> Result<T, DebugError> {
    let mut value: T = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<T>();
    
    let bytes_read = read_process_memory(
        process,
        address,
        unsafe { std::slice::from_raw_parts_mut(&mut value as *mut T as *mut u8, size) },
    )?;
    
    if bytes_read != size {
        return Err(DebugError::Other(format!(
            "read_remote_value: expected {} bytes, got {}",
            size, bytes_read
        )));
    }
    
    Ok(value)
}

pub fn get_entry_point_from_memory(process: HANDLE, image_base: Address) -> Result<Address, DebugError> {
    let dos_header: IMAGE_DOS_HEADER = read_remote_value(process, image_base)?;
    if dos_header.e_magic != 0x5A4D {
        return Err(DebugError::Other("Invalid DOS header".into()));
    }
    
    let nt_headers_addr = image_base.0 + dos_header.e_lfanew as usize;
    let nt_headers: IMAGE_NT_HEADERS64 = read_remote_value(process, Address(nt_headers_addr))?;
    
    let entry_point_rva = nt_headers.OptionalHeader.AddressOfEntryPoint as usize;
    
    Ok(Address(image_base.0 + entry_point_rva))
}