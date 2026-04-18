import Foundation

public struct EmittedFunction: Codable {
    public let name: String
    public let bodyLines: [String]
}

public final class BytecodeEmitter {
    public init() {}

    // Phase-2 placeholder emitter to bridge parser output into serializable units.
    public func emit(functionName: String, bodyLines: [String]) -> EmittedFunction {
        EmittedFunction(name: functionName, bodyLines: bodyLines)
    }
}
