// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "omx-menubar",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "OmxMenubarApp", targets: ["OmxMenubarApp"])
    ],
    targets: [
        .target(
            name: "OmxMenubarCore",
            linkerSettings: [
                .unsafeFlags(["../../target/release/libomx_menubar_ffi.a"])
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
