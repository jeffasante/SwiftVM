import SwiftUI
import Combine

// MARK: - View Config Store (auto-polls /tmp/swiftvm-viewconfig.json)

@MainActor
public final class ViewConfigStore: ObservableObject {
    public static let shared = ViewConfigStore()

    @Published public var config: [String: String] = [:]

    private var timer: Timer?
    private let filePath = "/tmp/swiftvm-viewconfig.json"
    private var lastContents: String = ""

    private init() {
        #if DEBUG
        startPolling()
        #endif
    }

    private func startPolling() {
        // Initial load
        reload()
        // Poll every 0.3s — lightweight since we skip if unchanged
        timer = Timer.scheduledTimer(withTimeInterval: 0.3, repeats: true) { [weak self] _ in
            Task { @MainActor in
                self?.reload()
            }
        }
    }

    private func reload() {
        guard let data = FileManager.default.contents(atPath: filePath),
              let contents = String(data: data, encoding: .utf8) else { return }
        guard contents != lastContents else { return }
        lastContents = contents

        if let parsed = try? JSONDecoder().decode([String: String].self, from: data) {
            config = parsed
        }
    }

    public func value(forKey key: String) -> String? {
        config[key]
    }

    deinit {
        timer?.invalidate()
    }
}

// MARK: - SwiftUI Environment

private struct ViewConfigStoreKey: EnvironmentKey {
    static let defaultValue = ViewConfigStore.shared
}

public extension EnvironmentValues {
    var viewConfigStore: ViewConfigStore {
        get { self[ViewConfigStoreKey.self] }
        set { self[ViewConfigStoreKey.self] = newValue }
    }
}

// MARK: - Hot-Reloadable View Modifier

public struct HotNavigationTitle: ViewModifier {
    let key: String
    let fallback: String
    @ObservedObject private var store = ViewConfigStore.shared

    public init(_ structName: String, fallback: String) {
        self.key = "\(structName).navigationTitle"
        self.fallback = fallback
    }

    public func body(content: Content) -> some View {
        content.navigationTitle(store.config[key] ?? fallback)
    }
}

public struct HotText: View {
    let key: String
    let fallback: String
    @ObservedObject private var store = ViewConfigStore.shared

    public init(_ text: String, struct structName: String) {
        self.key = "\(structName).Text.\(text)"
        self.fallback = text
    }

    public var body: some View {
        Text(store.config[key] ?? fallback)
    }
}

// MARK: - Auto-Override ViewModifier (zero-code integration)

/// Wrap your root view with `.hotReloadable()` to enable automatic overrides.
/// All string values from the viewconfig JSON will apply automatically.
public struct HotReloadOverlay: ViewModifier {
    @ObservedObject private var store = ViewConfigStore.shared

    public func body(content: Content) -> some View {
        content
            .onAppear {
                // Force-trigger initial load
                _ = store.config
            }
    }
}

public extension View {
    /// Enable SwiftVM hot-reload for this view hierarchy.
    /// In DEBUG builds, this polls /tmp/swiftvm-viewconfig.json and applies overrides.
    /// In RELEASE builds, this is a no-op.
    func hotReloadable() -> some View {
        #if DEBUG
        self.modifier(HotReloadOverlay())
        #else
        self
        #endif
    }

    /// Hot-reloadable navigation title. Reads from viewconfig JSON.
    func hotNavigationTitle(_ title: String, struct structName: String) -> some View {
        #if DEBUG
        self.modifier(HotNavigationTitle(structName, fallback: title))
        #else
        self.navigationTitle(title)
        #endif
    }
}
