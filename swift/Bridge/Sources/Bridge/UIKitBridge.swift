import Foundation

final class BridgeRegistry {
    static let shared = BridgeRegistry()

    private init() {}

    func invoke(
        selector: String,
        args: UnsafePointer<NativeValue>?,
        argCount: Int,
        outResult: UnsafeMutablePointer<NativeValue>
    ) -> Int32 {
        switch selector {
        case "debug.echo":
            if let args = args, argCount > 0 {
                outResult.pointee = args[0]
            } else {
                outResult.pointee = NativeValue()
            }
            return 0
        default:
            outResult.pointee = NativeValue()
            return -4
        }
    }
}
