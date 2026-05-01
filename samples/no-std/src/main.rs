#![feature(naked_functions_rustic_abi)]
#![no_std]
#![no_main]
#![windows_subsystem = "console"]
#![allow(unused_unsafe, unsafe_op_in_unsafe_fn)]

use core::{arch::{asm, naked_asm}, panic::PanicInfo};

#[cfg(not(test))]
#[inline(never)]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "C" fn mainCRTStartup() -> ! {
	naked_asm!(
		"nop",
	);
}