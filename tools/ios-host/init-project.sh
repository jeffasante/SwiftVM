#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<USAGE
Usage:
  init-project.sh \
    --project-root <path/to/project/root> \
    --xcodeproj <path/to/App.xcodeproj> \
    --scheme <Scheme> \
    --bundle-id <com.example.app> \
    --app-sources <path/to/app/source/folder>

Creates:
- <project-root>/run-sim.sh (wrapper)
- <project-root>/vm-content.swift (VM source seed)
- <app-sources>/SwiftVMBridge.swift (bridge scaffold)
USAGE
}

PROJECT_ROOT=""
XCODEPROJ=""
SCHEME=""
BUNDLE_ID=""
APP_SOURCES=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --project-root) PROJECT_ROOT="$2"; shift 2 ;;
    --xcodeproj) XCODEPROJ="$2"; shift 2 ;;
    --scheme) SCHEME="$2"; shift 2 ;;
    --bundle-id) BUNDLE_ID="$2"; shift 2 ;;
    --app-sources) APP_SOURCES="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; usage; exit 1 ;;
  esac
done

if [[ -z "$PROJECT_ROOT" || -z "$XCODEPROJ" || -z "$SCHEME" || -z "$BUNDLE_ID" || -z "$APP_SOURCES" ]]; then
  usage
  exit 1
fi

mkdir -p "$PROJECT_ROOT" "$APP_SOURCES"

cat > "$PROJECT_ROOT/run-sim.sh" <<RUNNER
#!/usr/bin/env bash
set -euo pipefail

ROOT="\$(cd "\$(dirname "\$0")" && pwd)"
TOOL_DIR="$(cd "$(dirname "$0")" && pwd)"
"\$TOOL_DIR/run-sim.sh" \
  --project "$XCODEPROJ" \
  --scheme "$SCHEME" \
  --bundle-id "$BUNDLE_ID"
RUNNER
chmod +x "$PROJECT_ROOT/run-sim.sh"

if [[ ! -f "$PROJECT_ROOT/vm-content.swift" ]]; then
  cat > "$PROJECT_ROOT/vm-content.swift" <<'VMFILE'
func main() -> String {
    return "Hello from Swift VM!|Edit vm-content.swift and wire your host to reload this output.|1"
}
VMFILE
fi

BRIDGE_FILE="$APP_SOURCES/SwiftVMBridge.swift"
if [[ ! -f "$BRIDGE_FILE" ]]; then
  cat > "$BRIDGE_FILE" <<'SWIFT'
import Foundation

@MainActor
final class SwiftVMBridge: ObservableObject {
    struct Output {
        let title: String
        let subtitle: String
        let count: Int
    }

    @Published var output: Output?

    /// Placeholder execution path.
    /// iOS apps cannot spawn arbitrary local processes in production,
    /// so this should eventually call an embedded VM or dev-only IPC service.
    func reload() async {
        let simulated = "Hello from Swift VM!|Bridge scaffold active. Wire this to embedded VM/IPC next.|1"
        output = parse(simulated)
    }

    private func parse(_ line: String) -> Output? {
        let parts = line.split(separator: "|", omittingEmptySubsequences: false).map(String.init)
        guard parts.count >= 3 else { return nil }
        return Output(title: parts[0], subtitle: parts[1], count: Int(parts[2]) ?? 0)
    }
}
SWIFT
fi

echo "Initialized SwiftVM host scaffolding in: $PROJECT_ROOT"
echo "- run-sim wrapper: $PROJECT_ROOT/run-sim.sh"
echo "- VM source seed:   $PROJECT_ROOT/vm-content.swift"
echo "- bridge scaffold:  $BRIDGE_FILE"
