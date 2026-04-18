import Foundation

public struct NativeValue {
    public var tag: UInt8
    public var intValue: Int64
    public var boolValue: UInt8
    public var stringPtr: UnsafePointer<CChar>?

    public init(tag: UInt8 = 3, intValue: Int64 = 0, boolValue: UInt8 = 0, stringPtr: UnsafePointer<CChar>? = nil) {
        self.tag = tag
        self.intValue = intValue
        self.boolValue = boolValue
        self.stringPtr = stringPtr
    }
}

@_cdecl("swift_bridge_call_native")
public func swiftBridgeCallNative(
    selector: UnsafePointer<CChar>,
    args: UnsafeRawPointer?,
    argCount: Int,
    outResult: UnsafeMutableRawPointer?
) -> Int32 {
    guard let outResult else {
        return -1
    }

    let typedArgs = args?.assumingMemoryBound(to: NativeValue.self)
    let typedOut = outResult.assumingMemoryBound(to: NativeValue.self)
    let sel = String(cString: selector)
    return BridgeRegistry.shared.invoke(selector: sel, args: typedArgs, argCount: argCount, outResult: typedOut)
}
