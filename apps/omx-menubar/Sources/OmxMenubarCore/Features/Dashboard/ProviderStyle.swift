import SwiftUI

/// Shared provider visual identity (icon + color), used by the tab bar, the
/// overview rows, and the dashboard. Single source of truth so a provider looks
/// the same everywhere.
enum ProviderStyle {
    static func icon(_ provider: String) -> String {
        switch provider.lowercased() {
        case "codex": return "brain.head.profile"
        case "claude": return "sparkle"
        case "gemini": return "diamond.fill"
        default: return "circle.grid.2x2.fill"
        }
    }

    static func color(_ provider: String) -> Color {
        switch provider.lowercased() {
        case "codex": return .green
        case "claude": return .orange
        case "gemini": return .blue
        default: return .purple
        }
    }

    /// The Overview tab's icon (the aggregated, all-providers view).
    static let overviewIcon = "square.grid.2x2.fill"
    static let overviewColor = Color.purple
}
