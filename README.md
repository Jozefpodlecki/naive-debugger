
# 🐞 Mini Debugger

![rustc](https://img.shields.io/badge/rustc-1.94.0-blue.svg)
![CI](https://github.com/Jozefpodlecki/naive-debugger/actions/workflows/ci.yml/badge.svg)

A Windows user-mode debugger written in Rust using Win32 debugging APIs
(`WaitForDebugEvent`, `ContinueDebugEvent`, `CreateProcessW`).

Supports event-driven execution control and basic debugging primitives.

## 🧪 Getting started

```rust
let path = "path/to/binary.exe";
let debugger = WindowsDebugger::spawn(path);

let debugger = WindowsDebugger::spawn("target.exe");

loop {
    let event = debugger.next();

    match event {
        _ => {}
    }
}
```

## 📦 Features

- Process creation under debugger
- Event-driven debug loop
