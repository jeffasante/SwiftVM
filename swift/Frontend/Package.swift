// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "Frontend",
    platforms: [.macOS(.v13)],
    products: [
        .library(name: "Frontend", targets: ["Frontend"]),
        .executable(name: "swiftvm-frontend", targets: ["swiftvm-frontend"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-syntax.git", branch: "main"),
    ],
    targets: [
        .target(name: "Frontend"),
        .executableTarget(
            name: "swiftvm-frontend",
            dependencies: [
                "Frontend",
                .product(name: "SwiftSyntax", package: "swift-syntax"),
                .product(name: "SwiftParser", package: "swift-syntax"),
            ]
        ),
    ]
)
