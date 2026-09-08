// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "AsyncProbe",
    platforms: [.macOS(.v13)],
    products: [.library(name: "AsyncProbe", type: .dynamic, targets: ["AsyncProbe"])],
    dependencies: [
        .package(url: "https://github.com/kabiroberai/node-swift.git", exact: "1.5.1"),
    ],
    targets: [
        .target(name: "AsyncProbe", dependencies: [
            .product(name: "NodeAPI", package: "node-swift"),
            .product(name: "NodeModuleSupport", package: "node-swift"),
        ], linkerSettings: [
            .unsafeFlags(["-Xlinker", "-undefined", "-Xlinker", "dynamic_lookup"]),
        ]),
    ],
    swiftLanguageModes: [.v5]
)
