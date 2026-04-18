# SwiftVM

![SwiftVM in Action](assets/how-to-use.gif)

A hot-reload engine for native SwiftUI development. Point it at an Xcode project, save a Swift file, and UI strings update in the iOS Simulator within about 100ms — no rebuild required.

## Highlights

- SwiftUI project hot-reload is now the primary workflow
- Run with ./dev.sh samples/swift/sample-food-truck-main
- Uses SwiftVMHook.swift plus SwiftVMHookBoot.m for auto-start
- Pure Rust extraction with incremental per-file cache
- Skips Xcode backup files and handles label key renames

## What it does

SwiftVM watches all `.swift` files in your project. On each save it extracts string literals from SwiftUI views using a pure-Rust parser (no subprocess, ~1ms), writes the result to `/tmp/swiftvm-viewconfig.json`, and a lightweight hook inside the running app polls that file every 100ms and patches the live UIKit view hierarchy in place.

The result is a Flutter-style hot-reload experience for native SwiftUI apps.

## Requirements

- macOS with Xcode and iOS Simulator
- Rust toolchain (`cargo`)
- Swift toolchain (comes with Xcode)

## Project structure

```
swift-dev-vm/
  crates/
    cli/          Rust CLI — file watcher, string extractor, dev server
    vm-core/      Bytecode virtual machine (single-file mode)
    hot-reload/   Differ, parser, and file watcher
    ffi-bridge/   C ABI bridge for native function calls
  swift/
    Frontend/     Swift-to-bytecode compiler (uses SwiftSyntax)
  tools/
    SwiftVMHook.swift     Drop-in hot-reload hook for any Xcode project
    SwiftVMHookBoot.m     ObjC constructor that auto-starts the hook
    inject-hook.rb        Injects the hook files into a .xcodeproj target
  samples/
    swift/
      sample-food-truck-main/   Apple WWDC22 Food Truck sample (pre-wired)
  apps/           Demo source files (.swift and .svm)
  dev.sh          One-command dev environment launcher
```

## Quick start (SwiftUI project mode)

### 1. Add the hook to your Xcode project (one time)

Copy `tools/SwiftVMHook.swift` and `tools/SwiftVMHookBoot.m` into your app target in Xcode. Both files are no-ops in Release builds and on device — they only activate in `DEBUG` Simulator builds.

`dev.sh` does this automatically for projects it recognises.

### 2. Build and run your app in Xcode

Press **Cmd+R** as normal. The hook starts polling as soon as the app launches.

### 3. Start the dev server

```bash
./dev.sh samples/swift/sample-food-truck-main
```

Or point it at any Xcode project directory:

```bash
./dev.sh path/to/MyApp
```

`dev.sh` builds the Rust engine, copies the hook files into the project if needed, and starts the watcher.

### 4. Edit and save

Change any string literal in your Swift source — a `.navigationTitle`, `Text(...)`, `Label(...)`, etc. — and save. The Simulator updates within ~100ms. No Xcode rebuild needed.

## How it works

1. On startup the CLI scans every `.swift` file in the project and builds an in-memory cache of all UI string literals.
2. The file watcher detects saves. Xcode backup files (`Foo~.swift`) are ignored.
3. Only the changed file is re-parsed (~1ms, pure Rust string extraction — no subprocess).
4. The updated JSON is written to `/tmp/swiftvm-viewconfig.json` only if content changed.
5. `SwiftVMHook` polls that file every 100ms. On a change it walks the live UIKit view hierarchy and patches matching `UILabel` text in place.
6. Key renames (e.g. changing `Label("Orders")` to `Label("My Orders")`) are detected as prefix matches and applied correctly.

## Supported SwiftUI string properties

The extractor recognises these eight patterns in any `.swift` file:

| Pattern | Key format |
|---|---|
| `.navigationTitle("…")` | `Struct.navigationTitle` |
| `.navigationSubtitle("…")` | `Struct.navigationSubtitle` |
| `Text("…")` | `Struct.Text.value` |
| `Label("…", …)` | `Struct.Label.value` |
| `.badge("…")` | `Struct.badge.value` |
| `.accessibilityLabel("…")` | `Struct.accessibilityLabel.value` |
| `.confirmationDialog("…")` | `Struct.confirmationDialog.value` |
| `.alert("…")` | `Struct.alert.value` |

## Adding the hook to a new project manually

If `dev.sh` doesn't auto-inject, add the files yourself:

1. Drag `tools/SwiftVMHook.swift` and `tools/SwiftVMHookBoot.m` into your app target in Xcode (tick "Add to target").
2. Build and run in the Simulator.
3. Run `./dev.sh path/to/YourProject`.

No other code changes are required. The hook self-starts via the ObjC `__attribute__((constructor))` in `SwiftVMHookBoot.m`.

## CLI controls

When the dev server is running:

| Key | Action |
|---|---|
| `r` | Light reload (recompile + patch, preserve state) |
| `R` | Hard reload (recompile + reset state) |
| `a` | Toggle auto-reload on save |
| `q` | Quit |

## Running without dev.sh

```bash
# Build the engine
cargo build -p swiftvm-cli

# Watch a project directory
cargo run -p swiftvm-cli -- path/to/MyApp

# Watch a single .swift or .svm file
cargo run -p swiftvm-cli -- apps/demo/main.svm
```

## Single-file mode (VM bytecode)

For `.svm` or simple `.swift` files the CLI also runs a full bytecode VM:

- Top-level `var` declarations (string, integer, boolean)
- `func name() -> Type { }` with local `let`/`var`, arithmetic, `if/else`, `for` loops
- `nativeCall("selector", args...)` bridge to native functions

State is written to `/tmp/swiftvm-state.json` and can be read from any iOS app via `SwiftVMBridge.swift`.

## Current limitations

- Hot-reload only works in the iOS Simulator (`/tmp` is shared between host and Simulator process). Device support would require a network transport.
- Adding or removing SwiftUI views still requires an Xcode rebuild. Only existing string literals can be patched live.
- The Rust extractor uses line-level string parsing. Multi-line string literals and string interpolation are not extracted.

## Running tests

```bash
cargo test
tests/integration/frontend_for_loop_regression.sh
tests/integration/native_bridge_once_regression.sh
tests/integration/object_props_regression.sh
tests/integration/logical_ops_regression.sh
```
