// swift-tools-version: 5.10
import PackageDescription

let rustTargetDir = Context.environment["CARGO_TARGET_DIR"] ?? "../../target"

let package = Package(
    name: "omx-menubar",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "OmxMenubarApp", targets: ["OmxMenubarApp"])
    ],
    targets: [
        .target(
            name: "OmxMenubarCore",
            resources: [.copy("Resources/ProviderIcons")],
            linkerSettings: [
                .unsafeFlags(["\(rustTargetDir)/release/libomx_menubar_ffi.a"])
            ]
        ),
        .executableTarget(
            name: "OmxMenubarApp",
            dependencies: ["OmxMenubarCore"]
        ),
        .executableTarget(
            name: "OmxMenubarContractTests",
            dependencies: ["OmxMenubarCore"]
        ),
    ]
)
