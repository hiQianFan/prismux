import SwiftUI

struct ProviderPage<Content: View>: View {
    let provider: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: PrismuxTokens.Spacing.md) {
            SectionHeader(title: provider.capitalized, subtitle: nil)
            content
        }
    }
}
