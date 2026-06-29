import SwiftUI

/// One row in the redesigned Overview: provider identity + active target +
/// lowest-quota meter + status dot. Tapping jumps to that provider's tab.
/// Replaces the old poolSummary + providerSummaries stat blocks.
struct OverviewProviderRow: View {
    let provider: String
    let activeTarget: String?
    let accountCount: Int
    let profileCount: Int
    let lowestQuotaPercent: Int?
    let statusText: String
    let statusTone: Color
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            HStack(spacing: OmxTokens.Spacing.md) {
                badge

                VStack(alignment: .leading, spacing: 2) {
                    Text(provider.capitalized)
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(.primary)
                    Text(subtitle)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }

                Spacer(minLength: 8)

                quota

                Image(systemName: "chevron.right")
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(.tertiary)
            }
            .padding(.vertical, 8)
            .padding(.horizontal, 4)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private var badge: some View {
        ZStack {
            Circle().fill(ProviderStyle.color(provider).opacity(0.18))
            ProviderIcon(provider: provider, size: 13)
                .foregroundStyle(ProviderStyle.color(provider))
        }
        .frame(width: 30, height: 30)
    }

    @ViewBuilder
    private var quota: some View {
        if let percent = lowestQuotaPercent {
            VStack(alignment: .trailing, spacing: 3) {
                Text("\(percent)%")
                    .font(.caption.monospacedDigit().weight(.semibold))
                    .foregroundStyle(quotaColor(percent))
                quotaBar(percent)
            }
            .frame(width: 56)
        } else {
            statusDot
        }
    }

    private func quotaBar(_ percent: Int) -> some View {
        GeometryReader { proxy in
            ZStack(alignment: .leading) {
                Capsule().fill(Color.primary.opacity(0.12))
                Capsule()
                    .fill(quotaColor(percent))
                    .frame(width: max(2, proxy.size.width * CGFloat(percent) / 100))
            }
        }
        .frame(height: 4)
    }

    private var statusDot: some View {
        HStack(spacing: 4) {
            Circle().fill(statusTone).frame(width: 7, height: 7)
            Text(statusText)
                .font(.caption2)
                .foregroundStyle(.secondary)
                .lineLimit(1)
        }
        .frame(width: 56, alignment: .trailing)
    }

    private var subtitle: String {
        if let activeTarget, !activeTarget.isEmpty, activeTarget != "-" {
            return activeTarget
        }
        let accounts = accountCount == 0 ? "No accounts" : "\(accountCount) account\(accountCount == 1 ? "" : "s")"
        if profileCount == 0 { return accounts }
        return "\(accounts) · \(profileCount) profile\(profileCount == 1 ? "" : "s")"
    }

    private func quotaColor(_ percent: Int) -> Color {
        if percent <= 15 { return .red }
        if percent <= 40 { return .orange }
        return .green
    }
}
