import SwiftUI

/// One provider's health card: identity + inventory (account/profile counts) +
/// current active target + 5h/7d average-remaining bars. Reused by both the
/// Overview (one card per provider, tappable to jump) and a provider page (the
/// single card for that provider) — same component, different data.
///
/// The 5h/7d bars render through the shared `QuotaBar`, so they're pixel-
/// identical to the account card's quota lines. Values are the backend's
/// per-window-class averages; this view computes no aggregation.
struct ProviderSummaryCard: View {
    let aggregate: ProviderAggregateView
    /// Masked active label, already run through privacy logic by the caller.
    let activeLabel: String?
    /// When set, the whole card is a button that jumps to this provider's tab.
    /// nil on the provider page (already there).
    var onTap: (() -> Void)?

    @Environment(\.colorScheme) private var colorScheme

    private var provider: String { aggregate.providerId }
    private var windows: WindowAverages? { aggregate.quotaHealth.windowAverages }

    var body: some View {
        if let onTap {
            Button(action: onTap) { card }
                .buttonStyle(.plain)
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(accessibilitySummary)
                .accessibilityHint("Open \(provider.capitalized) tab")
        } else {
            card
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(accessibilitySummary)
        }
    }

    private var card: some View {
        VStack(alignment: .leading, spacing: 8) {
            header
            if hasWindows {
                VStack(alignment: .leading, spacing: 4) {
                    QuotaBar(label: "5h", remainingPercentX100: windows?.shortRemainingPercentX100)
                    QuotaBar(label: "7d", remainingPercentX100: windows?.weeklyRemainingPercentX100)
                }
            } else {
                Text("No quota reported")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.vertical, 8)
        .padding(.horizontal, 4)
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
    }

    private var header: some View {
        HStack(spacing: 8) {
            badge
            VStack(alignment: .leading, spacing: 2) {
                Text(provider.capitalized)
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.primary)
                Text(inventoryText)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            Spacer(minLength: 8)
            Text(routingText)
                .font(.caption.monospacedDigit())
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
        }
    }

    private var badge: some View {
        ZStack {
            Circle().fill(ProviderStyle.color(provider).opacity(0.18))
            ProviderIcon(provider: provider, size: 12)
                .foregroundStyle(ProviderStyle.color(provider))
        }
        .frame(width: 26, height: 26)
    }

    private var hasWindows: Bool {
        windows?.shortRemainingPercentX100 != nil || windows?.weeklyRemainingPercentX100 != nil
    }

    private var inventoryText: String {
        let accounts = Int(aggregate.accountCount)
        let profiles = Int(aggregate.profileCount)
        var parts: [String] = []
        if accounts > 0 || profiles == 0 {
            parts.append("\(accounts) acct\(accounts == 1 ? "" : "s")")
        }
        if profiles > 0 {
            parts.append("\(profiles) prof")
        }
        return parts.joined(separator: " · ")
    }

    private var routingText: String {
        guard let activeLabel, !activeLabel.isEmpty, activeLabel != "-" else {
            return "→ none"
        }
        return "→ \(activeLabel)"
    }

    private var accessibilitySummary: String {
        var parts = [provider.capitalized, inventoryText, "active \(routingText.replacingOccurrences(of: "→ ", with: ""))"]
        if let short = windows?.shortRemainingPercentX100 {
            parts.append("5 hour \(Int(short) / 100) percent")
        }
        if let weekly = windows?.weeklyRemainingPercentX100 {
            parts.append("7 day \(Int(weekly) / 100) percent")
        }
        return parts.joined(separator: ", ")
    }
}
