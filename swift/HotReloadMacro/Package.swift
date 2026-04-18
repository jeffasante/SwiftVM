// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "HotReloadMacro",
    platforms: [.macOS(.v13)],
    products: [
        .library(name: "HotReloadMacro", targets: ["HotReloadMacro"]),
    ],
    targets: [
        .target(name: "HotReloadMacro"),
    ]
)
