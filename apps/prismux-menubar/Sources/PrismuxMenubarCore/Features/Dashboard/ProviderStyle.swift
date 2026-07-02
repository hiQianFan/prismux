import AppKit
import SwiftUI

/// Shared provider visual identity (icon + color), used by the tab bar, the
/// overview rows, and the dashboard. Single source of truth so a provider looks
/// the same everywhere.
enum ProviderStyle {
    enum IconSource {
        case asset(String)
        case system(String)
    }

    struct HSB {
        let hue: Double
        let saturation: Double
        let brightness: Double
    }

    static func icon(_ provider: String?) -> IconSource {
        switch provider?.lowercased() {
        case "codex": return .asset("codex")
        case "claude": return .asset("claude")
        case "gemini": return .system("diamond.fill")
        default: return .system("circle.grid.2x2.fill")
        }
    }

    static func color(_ provider: String) -> Color {
        let value = hsb(provider)
        return Color(hue: value.hue, saturation: value.saturation, brightness: value.brightness)
    }

    static func hsb(_ provider: String?) -> HSB {
        switch provider?.lowercased() {
        // Official brand colors: Codex/ChatGPT green #10A37F, Claude/Anthropic
        // "book cloth" terracotta #CC785C, Gemini Google blue #4285F4. These
        // are desaturated brand tones that sit together far more calmly than
        // the old system green+orange.
        case "codex": return HSB(hue: 0.455, saturation: 0.90, brightness: 0.64) // #10A37F
        case "claude": return HSB(hue: 0.042, saturation: 0.55, brightness: 0.80) // #CC785C
        case "gemini": return HSB(hue: 0.604, saturation: 0.73, brightness: 0.96) // #4285F4
        default: return HSB(hue: 0.75, saturation: 0.58, brightness: 0.72)
        }
    }

    /// The Overview tab's icon (the aggregated, all-providers view).
    static let overviewIcon = "square.grid.2x2.fill"
    static let overviewColor = Color.purple
}

struct ProviderIcon: View {
    let provider: String?
    let size: CGFloat
    var weight: Font.Weight = .semibold

    var body: some View {
        switch ProviderStyle.icon(provider) {
        case let .asset(name):
            Image(nsImage: Self.assetImage(name))
                .resizable()
                .renderingMode(.template)
                .scaledToFit()
                .frame(width: size, height: size)
        case let .system(symbol):
            Image(systemName: symbol)
                .font(.system(size: size, weight: weight))
        }
    }

    private static func assetImage(_ name: String) -> NSImage {
        guard let url = iconURL(name), let image = NSImage(contentsOf: url) else {
            // Degrade to a blank template rather than crashing: a missing icon
            // asset must never take down the whole menu bar app.
            assertionFailure("Missing provider icon resource: \(name).svg")
            return NSImage(size: NSSize(width: 1, height: 1))
        }
        image.isTemplate = true
        return image
    }

    /// Resolve `ProviderIcons/<name>.svg` by direct filesystem lookup instead of
    /// the SwiftPM-generated `Bundle.module`, whose accessor calls `fatalError()`
    /// (SIGTRAP) when it can't locate the bundle — a hard crash that has taken
    /// down the app mid-render on downloaded/translocated copies. We probe the
    /// resource roots for both the flat bundle layout produced by the SwiftPM CLI
    /// build (`X.bundle/ProviderIcons`) and the nested macOS layout produced by
    /// the Xcode build backend (`X.bundle/Contents/Resources/ProviderIcons`).
    private static func iconURL(_ name: String) -> URL? {
        let bundleName = "prismux-menubar_PrismuxMenubarCore"
        let file = "\(name).svg"
        let roots = [
            Bundle.main.resourceURL,
            Bundle(for: BundleToken.self).resourceURL,
            Bundle.main.bundleURL.appendingPathComponent("Contents/Resources"),
        ].compactMap { $0 }

        let relativePaths = [
            "\(bundleName).bundle/ProviderIcons/\(file)",
            "\(bundleName).bundle/Contents/Resources/ProviderIcons/\(file)",
            "ProviderIcons/\(file)",
        ]

        let fileManager = FileManager.default
        for root in roots {
            for relative in relativePaths {
                let candidate = root.appendingPathComponent(relative)
                if fileManager.fileExists(atPath: candidate.path) {
                    return candidate
                }
            }
        }
        return nil
    }

    private final class BundleToken {}
}
