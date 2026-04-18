// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "Bridge",
    platforms: [.macOS(.v13)],
    products: [
        .library(name: "Bridge", targets: ["Bridge"]),
    ],
    targets: [
        .target(name: "Bridge"),
    ]
)
