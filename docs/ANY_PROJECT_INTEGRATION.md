# Any Project Integration

This repo now includes reusable tools to integrate SwiftVM dev flow into any iOS host project.

## 1) Initialize a project scaffold

```bash
./tools/ios-host/init-project.sh \
  --project-root path/to/your/project \
  --xcodeproj path/to/your/project/YourApp.xcodeproj \
  --scheme YourApp \
  --bundle-id com.example.yourapp \
  --app-sources path/to/your/project/YourApp
```

This creates:

- `run-sim.sh` wrapper in your project root
- `vm-content.swift` seed file
- `SwiftVMBridge.swift` scaffold in app sources

## 2) Build/install/launch in one command

```bash
./run-sim.sh
```

## 3) Wire UI to bridge

In your SwiftUI view, add a `@StateObject var bridge = SwiftVMBridge()` and call `await bridge.reload()` from `.onAppear` and a button.

Use `bridge.output` to render title/subtitle/count.

## 4) Current limitation

This is a dev scaffold. For true hot reload in app process, replace the simulated output path inside `SwiftVMBridge.reload()` with:

- embedded VM runtime call, or
- dev-only IPC transport to a local VM process.
