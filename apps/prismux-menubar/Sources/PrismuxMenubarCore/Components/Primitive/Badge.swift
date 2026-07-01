import SwiftUI

struct Badge: View {
    let text: String
    let color: Color

    var body: some View {
        Text(text)
            .font(.caption2.weight(.semibold))
            .padding(.horizontal, PrismuxTokens.Spacing.sm)
            .padding(.vertical, PrismuxTokens.Spacing.xs)
            .background(color.opacity(0.12), in: Capsule())
            .foregroundStyle(color)
    }
}
