pub fn infinite_loop() {
    loop {}
}

pub fn infinite_sleep_loop() {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

pub fn sleep() {
    std::thread::sleep(std::time::Duration::from_secs(60));
}
