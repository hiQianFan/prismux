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
        // Official brand colors: Codex/ChatGPT green #10A37F, Claude/Anthropic
        // "book cloth" terracotta #CC785C, Gemini Google blue #4285F4. These
        // are desaturated brand tones that sit together far more calmly than
        // the old system green+orange.
        case "codex": return Color(red: 0.063, green: 0.639, blue: 0.498) // #10A37F
        case "claude": return Color(red: 0.800, green: 0.471, blue: 0.361) // #CC785C
        case "gemini": return Color(red: 0.259, green: 0.522, blue: 0.957) // #4285F4
        default: return .purple
        }
    }

    /// The Overview tab's icon (the aggregated, all-providers view).
    static let overviewIcon = "square.grid.2x2.fill"
    static let overviewColor = Color.purple
}
