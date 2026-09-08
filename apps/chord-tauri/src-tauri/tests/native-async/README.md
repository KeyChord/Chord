Run this macOS regression check from `src-tauri`:

```sh
cargo build --bin chord
swift build --package-path tests/native-async --scratch-path target/native-async --product AsyncProbe
python3 tests/native-async/check.py target/debug/chord target/native-async/debug/libAsyncProbe.dylib
```

The fixture checks repeated NodeSwift async → MainActor → NodeActor transitions,
actual main-thread execution, Promise rejection and recovery, and nonzero CLI
exit for an uncaught Swift error. Each CLI invocation has a ten-second timeout
so main-thread starvation fails the check instead of hanging it.

`check.py` can also accept an addon compiled from `Sources/AsyncProbe/probe.swift`
against an existing NodeSwift build. It loads the native library directly; the
file extension does not affect `process.dlopen`.
