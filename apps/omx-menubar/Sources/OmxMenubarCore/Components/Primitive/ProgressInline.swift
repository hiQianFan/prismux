import SwiftUI

struct ProgressInline: View {
    let text: String

    var body: some View {
        HStack(spacing: OmxTokens.Spacing.sm) {
            ProgressView()
                .controlSize(.small)
            Text(text)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }
}
