pub fn infinite_loop() {
    loop {}
}

pub fn sleep() {
    std::thread::sleep(std::time::Duration::from_secs(60));
}
