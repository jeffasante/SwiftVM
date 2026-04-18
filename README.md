# SwiftVM

https://github.com/jeffasante/SwiftVM/raw/main/assets/how-to-use.mp4

A hot-reload engine for native iOS development. Edit Swift logic and UI properties in a text file and see changes appear in the iOS Simulator instantly, without rebuilding the app.

## What it does

SwiftVM watches a Swift source file for changes. When you save the file, the engine recompiles it into bytecode, runs it in a lightweight Rust VM, and pushes the updated state to your running iOS app through a shared JSON file. The app picks up the new state within 300ms and the UI updates automatically.

This gives you a workflow similar to Flutter hot-reload, but for native SwiftUI apps.

## Requirements

- macOS with Xcode and iOS Simulator
- Rust toolchain (cargo)
- Swift toolchain (comes with Xcode)

## Project structure

```
swift-dev-vm/
  crates/
    cli/          Rust CLI that runs the VM and watches for file changes
    vm-core/      The bytecode virtual machine
    hot-reload/   Differ, parser, and file watcher
    ffi-bridge/   C ABI bridge for native function calls
  swift/
    Frontend/     Swift-to-bytecode compiler (uses SwiftSyntax)
  test-reload/    Example iOS app wired to the VM bridge
  apps/           Demo source files (.swift and .svm)
  dev.sh          One-command script to build and run everything
```

## Quick start

### 1. Build the Swift frontend (first time only)

```bash
xcrun swift build --package-path swift/Frontend
```

### 2. Run the dev environment

```bash
./dev.sh
```

This script does three things:
1. Builds the Rust VM engine
2. Builds and installs the iOS app on the Simulator
3. Starts the hot-reload dev server watching your source file

### 3. Edit and save

Open `test-reload/test-reload/Logic.swift` in your editor. Change any value and save. The Simulator updates automatically.

## How the source file works

The VM watches a single Swift file. This file contains two kinds of declarations:

**State variables** define values the iOS app can read:

```swift
var titleText = "My App"
var titleColor = "blue"
var padding = "30"
var count = 42
```

**Functions** define logic the VM executes:

```swift
func main() -> String {
    return titleText
}
```

Every variable you declare becomes available in the iOS app through the bridge. You do not need to register them anywhere. Just add a `var` line, save the file, and read it from the app.

## Using the bridge in your iOS app

### 1. Add the bridge file

Copy `SwiftVMBridge.swift` into your Xcode project. This is the only file you need from this repo.

### 2. Create the bridge in your App struct

```swift
import SwiftUI

@main
struct MyApp: App {
    @StateObject private var vm = SwiftVMBridge()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(vm)
        }
    }
}
```

### 3. Read values in your views

```swift
struct ContentView: View {
    @EnvironmentObject var vm: SwiftVMBridge

    var body: some View {
        VStack(spacing: CGFloat(vm.int("spacing", default: 20))) {
            Text(vm.string("titleText"))
                .font(.system(size: CGFloat(vm.int("titleSize", default: 28))))

            Text(vm.string("subtitle"))
                .foregroundColor(.secondary)

            Text("Count: \(vm.int("count"))")
        }
        .padding(CGFloat(vm.int("padding", default: 20)))
    }
}
```

### Available accessors

| Method | Returns | Example |
|---|---|---|
| `vm.string("key")` | String | `vm.string("title")` |
| `vm.string("key", default: "fallback")` | String | `vm.string("title", default: "Hello")` |
| `vm.int("key")` | Int | `vm.int("count")` |
| `vm.int("key", default: 0)` | Int | `vm.int("padding", default: 20)` |
| `vm.double("key")` | Double | `vm.double("opacity")` |
| `vm.double("key", default: 1.0)` | Double | `vm.double("opacity", default: 1.0)` |

The bridge polls `/tmp/swiftvm-state.json` every 300ms. When values change, `@Published` triggers a SwiftUI view update. You do not need to call reload manually.

## CLI controls

When the dev server is running, you can type these keys:

| Key | Action |
|---|---|
| `r` | Light reload. Recompiles the source and patches changed functions. Preserves state. |
| `R` | Hard reload. Recompiles and resets all state to defaults. |
| `a` | Toggle auto-reload on file save. On by default. |
| `q` | Quit the dev server. |

## Running without the dev script

If you prefer to run things separately:

**Build the engine:**

```bash
cargo build -p swiftvm-cli
```

**Install and launch the iOS app:**

```bash
test-reload/run-sim.sh
```

**Start the dev server:**

```bash
cargo run -p swiftvm-cli -- test-reload/test-reload/Logic.swift
```

## Supported Swift syntax in source files

The Swift frontend compiles a subset of Swift into VM bytecode:

- Top-level `var` declarations with string, integer, or boolean values
- Functions with `func name() -> Type { }` syntax
- Local `let` and `var` bindings
- Assignment (`name = expr`)
- Arithmetic and comparison operators (`+ - * / == < > <= >=`)
- Logical operators (`&& ||`)
- `if / else` control flow
- `for i in a..<b` and `for i in a...b` range loops
- Function calls with arguments
- `String()` type conversion
- Native bridge calls with `nativeCall("selector", args...)`

## How it works internally

1. The CLI reads your `.swift` file and passes it to `swiftvm-frontend`.
2. The frontend uses SwiftSyntax to parse the file and emit bytecode instructions.
3. The CLI loads the bytecode into the Rust VM and executes the `main` function.
4. After each execution tick, the CLI writes all VM global variables to `/tmp/swiftvm-state.json` as a JSON object.
5. The iOS app polls that file every 300ms and updates `@Published` state when it detects a change.
6. SwiftUI re-renders any views that depend on the changed values.

When you save the source file, the file watcher triggers a recompile. The hot-reload differ compares the old and new bytecode programs. If only function bodies or state values changed, it performs a light reload (patches in place, preserves live state). If function signatures changed, it requires a hard reload.

## Current limitations

- The bridge uses a file on disk (`/tmp`), which only works in the iOS Simulator. A device build would need a network or embedded VM approach.
- The Swift frontend supports a subset of Swift, not the full language.
- SwiftUI view structure (adding or removing views) still requires an Xcode rebuild. Only data-driven properties (text, colors, sizes, counts) can be hot-reloaded.
- The VM does not yet support closures, classes, or protocol conformances.

## Running tests

```bash
cargo test
tests/integration/frontend_for_loop_regression.sh
tests/integration/native_bridge_once_regression.sh
tests/integration/object_props_regression.sh
tests/integration/logical_ops_regression.sh
```
