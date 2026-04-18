import Foundation

public protocol StateSnapshotable {
    func snapshot() -> [String: Any]
    static func restore(from snapshot: [String: Any]) -> Self
}

// Phase-4 placeholder surface. Actual macro expansion wiring is pending.
@propertyWrapper
public struct HotReloadable<Value> {
    public var wrappedValue: Value

    public init(wrappedValue: Value) {
        self.wrappedValue = wrappedValue
    }
}
