import Foundation

public final class DependencyGraph {
    private var edges: [String: Set<String>] = [:]

    public init() {}

    public func setDependencies(file: String, dependsOn: Set<String>) {
        edges[file] = dependsOn
    }

    public func dependencies(of file: String) -> Set<String> {
        edges[file, default: []]
    }

    public func reverseDependents(of file: String) -> Set<String> {
        var result: Set<String> = []
        for (source, deps) in edges where deps.contains(file) {
            result.insert(source)
        }
        return result
    }
}
