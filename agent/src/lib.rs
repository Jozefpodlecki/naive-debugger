use windows_sys::Win32::Foundation::HINSTANCE;

#[unsafe(no_mangle)]
#[allow(non_snake_case, unused_variables)]
extern "system" fn DllMain(
    dll_module: HINSTANCE,
    call_reason: u32,
    _: *mut ())
    -> bool {
// {
//     match call_reason {
//         DLL_PROCESS_ATTACH => attach(),
//         DLL_PROCESS_DETACH => detach(),
//         _ => ()
//     }

    true
}