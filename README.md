
# 🐞 Naive Debugger

![rustc](https://img.shields.io/badge/rustc-1.97.0-blue.svg)
![CI](https://github.com/Jozefpodlecki/naive-debugger/actions/workflows/ci.yml/badge.svg)

A Windows user-mode debugger written in Rust using Win32 debugging APIs
(`WaitForDebugEvent`, `ContinueDebugEvent`, `CreateProcessW`).

Supports event-driven execution control and basic debugging primitives.

## 🧪 Getting started

```rust
let path = "path/to/binary.exe";
let mut debugger = WindowsDebugger
    ::spawn_with_options(exe_path, DebuggerOptions { single_step: true })
    .unwrap();

while let Ok(event) = debugger.next_event() {
    let context = event.context;

    let status = match event.kind {
        event => {
            println!("{:?}", event);
            ContinueStatus::Continue
        },
    };

    if let Err(err) = debugger.continue_event(context, status) {
        println!("{}", err);
        break;
    }
}
```

## 🏃‍♂️ Running Test Binaries

The workspace includes test binaries with different behaviors:

```sh
$env:EXAMPLE="infinite-sleep" 
cargo run -p with-std-examples
# or cargo run -p with-std-examples infinite-sleep
```

## 📦 Features

- **Process Creation & Attachment** – Spawn processes under debugger or attach to running ones
- **Software Breakpoints (INT3)** – Set, remove, disable, and re-enable breakpoints
- **Instruction-Level Single Stepping** – Execute one instruction at a time via trap flag
- **x64 Disassembly** – Powered by `iced-x86` for instruction decoding and display
- **Event Logging** – Structured logging via `log` crate with `flexi_logger` support