// swift-tools-version: 5.10
import PackageDescription

let rustTargetDir = Context.environment["CARGO_TARGET_DIR"] ?? "../../target"

let package = Package(
    name: "prismux-menubar",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "PrismuxMenubarApp", targets: ["PrismuxMenubarApp"])
    ],
    targets: [
        .target(
            name: "PrismuxMenubarCore",
            resources: [.copy("Resources/ProviderIcons")],
            linkerSettings: [
                .unsafeFlags(["\(rustTargetDir)/release/libprismux_menubar_ffi.a"])
            ]
        ),
        .executableTarget(
            name: "PrismuxMenubarApp",
            dependencies: ["PrismuxMenubarCore"]
        ),
        .executableTarget(
            name: "PrismuxMenubarContractTests",
            dependencies: ["PrismuxMenubarCore"]
        ),
    ]
)
