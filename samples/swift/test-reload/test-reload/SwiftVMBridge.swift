import Foundation

@MainActor
final class SwiftVMBridge: ObservableObject {
    @Published private var state: [String: String] = [:]
    private var lastLoadedContent: String = ""

    init() {
        startAutomaticRefresh()
    }

    /// Access a string from the VM state
    func string(_ key: String, default: String = "") -> String {
        return state[key] ?? `default`
    }

    /// Access an integer from the VM state
    func int(_ key: String, default: Int = 0) -> Int {
        return Int(state[key] ?? "") ?? `default`
    }

    /// Access a double from the VM state
    func double(_ key: String, default: Double = 0.0) -> Double {
        return Double(state[key] ?? "") ?? `default`
    }

    private func startAutomaticRefresh() {
        Timer.scheduledTimer(withTimeInterval: 0.3, repeats: true) { _ in
            Task { @MainActor in
                await self.checkForUpdates()
            }
        }
    }

    private func checkForUpdates() async {
        let bridgePath = "/tmp/swiftvm-state.json"
        guard let data = try? Data(contentsOf: URL(fileURLWithPath: bridgePath)) else { return }
        
        if let json = try? JSONSerialization.jsonObject(with: data) as? [String: String] {
            if json != state {
                self.state = json
                print("Bridge: UI State Updated")
            }
        }
    }

    func reload() async {
        await checkForUpdates()
    }
}
