// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "HotReloadBridge",
    platforms: [.iOS(.v16), .macOS(.v13)],
    products: [
        .library(name: "HotReloadBridge", targets: ["HotReloadBridge"]),
    ],
    targets: [
        .target(name: "HotReloadBridge"),
    ]
)
