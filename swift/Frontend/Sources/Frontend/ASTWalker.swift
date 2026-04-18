import Foundation

public final class ASTWalker {
    public init() {}

    // Phase-2 placeholder: currently operates on a simple line model.
    // Will be replaced with SwiftSyntax visitor traversal.
    public func collectFunctionNames(from source: String) -> [String] {
        source
            .split(separator: "\n")
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .compactMap { line in
                guard line.hasPrefix("func ") else { return nil }
                let remainder = line.dropFirst(5)
                guard let paren = remainder.firstIndex(of: "(") else { return nil }
                return String(remainder[..<paren]).trimmingCharacters(in: .whitespaces)
            }
    }
}
