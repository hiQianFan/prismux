import SwiftUI

/// Icon-only tab bar: an Overview tab plus one tab per provider. The active tab
/// is marked by a sliding pill (matchedGeometryEffect). `nil` selection = the
/// Overview tab, matching `AppStore.selectedProvider`.
struct ProviderTabBar: View {
    let providers: [String]
    let selected: String?
    let onSelect: (String?) -> Void

    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        HStack(spacing: 4) {
            tab(provider: nil)
            ForEach(providers, id: \.self) { provider in
                tab(provider: provider)
            }
            Spacer(minLength: 0)
        }
    }

    private func tab(provider: String?) -> some View {
        let isSelected = provider == selected
        let tint = provider.map(ProviderStyle.color) ?? ProviderStyle.overviewColor
        let label = provider?.capitalized ?? "Overview"

        return Button {
            guard provider != selected else { return }
            if reduceMotion {
                onSelect(provider)
            } else {
                withAnimation(.smooth(duration: 0.24)) { onSelect(provider) }
            }
        } label: {
            ProviderIcon(provider: provider, size: 14)
                .foregroundStyle(isSelected ? tint : Color.secondary)
                .frame(width: 38, height: 30)
                .background(
                    RoundedRectangle(cornerRadius: 8)
                        .fill(tint.opacity(isSelected ? 0.16 : 0))
                )
                .contentShape(RoundedRectangle(cornerRadius: 8))
        }
        .buttonStyle(.plain)
        .help(label)
        .accessibilityLabel(label)
        .accessibilityAddTraits(isSelected ? [.isSelected] : [])
    }
}
