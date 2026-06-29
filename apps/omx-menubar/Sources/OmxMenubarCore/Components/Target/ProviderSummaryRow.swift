import SwiftUI

/// One provider's row in the redesigned Overview. Three stacked lines, each
/// answering a question in action-priority order:
///   1. identity · avg remaining % · tone bar · healthy/low/exhausted counts
///   2. which account is routed right now (+ reset countdown when not all healthy)
///   3. this provider's tokens + cost for the selected period
///
/// All numbers come from the backend aggregate (`ProviderAggregateView`); this
/// view computes no thresholds and no aggregation. Tapping jumps to the
/// provider's tab.
struct ProviderSummaryRow: View {
    let aggregate: ProviderAggregateView
    /// Masked active label, already run through the privacy logic by the caller.
    let activeLabel: String?
    let onTap: () -> Void

    private var provider: String { aggregate.providerId }
    private var health: QuotaHealthRollup { aggregate.quotaHealth }

    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 5) {
                capacityLine
                routingLine
                usageLine
            }
            .padding(.vertical, 8)
            .padding(.horizontal, 4)
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(accessibilitySummary)
        .accessibilityHint("Open \(provider.capitalized) tab")
    }

    // MARK: Line 1 — capacity

    private var capacityLine: some View {
        HStack(spacing: 8) {
            badge
            Text(provider.capitalized)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.primary)

            Text(avgText)
                .font(.caption.monospacedDigit().weight(.semibold))
                .foregroundStyle(toneColor)

            toneBar

            Spacer(minLength: 8)

            Text(countsText)
                .font(.caption2)
                .foregroundStyle(.secondary)
                .lineLimit(1)
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

    @ViewBuilder
    private var toneBar: some View {
        if let fraction = avgFraction {
            GeometryReader { proxy in
                ZStack(alignment: .leading) {
                    Capsule().fill(Color.primary.opacity(0.12))
                    Capsule()
                        .fill(toneColor)
                        .frame(width: max(3, proxy.size.width * fraction))
                }
            }
            .frame(width: 48, height: 4)
        }
    }

    // MARK: Line 2 — routing

    private var routingLine: some View {
        HStack(spacing: 4) {
            Text("→")
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.tertiary)
            Text(routingText)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
            if let reset = resetText {
                Text("· \(reset)")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(toneColor)
                    .lineLimit(1)
            }
        }
    }

    // MARK: Line 3 — usage

    private var usageLine: some View {
        Text(usageText)
            .font(.caption.monospacedDigit())
            .foregroundStyle(.secondary)
            .lineLimit(1)
    }

    // MARK: Derived display values

    private var avgText: String {
        guard let avg = health.facts.avgRemainingPercentX100 else { return "— avg" }
        return "\(Int(avg) / 100)% avg"
    }

    private var avgFraction: Double? {
        guard let avg = health.facts.avgRemainingPercentX100 else { return nil }
        return max(0, min(1, Double(avg) / 10_000.0))
    }

    /// Brand-neutral health tone from the backend, paired with text counts so
    /// color is never the only signal.
    private var toneColor: Color {
        switch health.statusTone {
        case "success": return OmxTokens.StatusColor.healthy
        case "warning": return OmxTokens.StatusColor.warning
        case "danger": return OmxTokens.StatusColor.failed
        default: return OmxTokens.StatusColor.muted
        }
    }

    /// "3 · 1 low", zero segments omitted; all-healthy collapses to "N healthy".
    private var countsText: String {
        let healthy = Int(health.healthyCount)
        let low = Int(health.lowCount)
        let exhausted = Int(health.exhaustedCount)
        let total = healthy + low + exhausted
        if total == 0 { return "no quota" }
        if low == 0 && exhausted == 0 { return "\(healthy) healthy" }
        var parts = ["\(total)"]
        if low > 0 { parts.append("\(low) low") }
        if exhausted > 0 { parts.append("\(exhausted) exhausted") }
        return parts.joined(separator: " · ")
    }

    private var routingText: String {
        guard let activeLabel, !activeLabel.isEmpty, activeLabel != "-" else {
            return "no active target"
        }
        return activeLabel
    }

    /// Reset countdown only when something needs attention (low/exhausted > 0).
    private var resetText: String? {
        guard health.lowCount > 0 || health.exhaustedCount > 0 else { return nil }
        guard let reset = health.facts.soonestResetAtUnix else { return nil }
        return resetCountdown(reset)
    }

    /// token + cost, cost degraded by status. Missing cost is omitted, never $0.
    private var usageText: String {
        guard let headline = aggregate.usageHeadline else { return "—" }
        let tokens = tokenText(headline.totalTokens)
        guard let cost = costText(headline) else {
            return "\(tokens) tokens"
        }
        return "\(tokens) tokens · \(cost)"
    }

    private func costText(_ headline: UsageHeadline) -> String? {
        guard let raw = headline.estimatedCostUsd, let value = Double(raw) else { return nil }
        let amount = String(format: "$%.2f", value)
        switch headline.costStatus {
        case "Missing": return nil
        case "ProviderReported": return amount
        case "Estimated": return "~\(amount) est."
        case "Mixed": return "~\(amount)"
        default: return "~\(amount)"
        }
    }

    private var accessibilitySummary: String {
        var parts = [provider.capitalized]
        if let avg = health.facts.avgRemainingPercentX100 {
            parts.append("average remaining \(Int(avg) / 100) percent")
        } else {
            parts.append("no quota reported")
        }
        parts.append(countsText)
        parts.append("active: \(routingText)")
        if let reset = resetText { parts.append(reset) }
        if let headline = aggregate.usageHeadline {
            parts.append("\(tokenText(headline.totalTokens)) tokens this period")
        }
        return parts.joined(separator: ", ")
    }

    private func tokenText(_ tokens: UInt64) -> String {
        if tokens >= 1_000_000 { return String(format: "%.1fM", Double(tokens) / 1_000_000) }
        if tokens >= 1_000 { return String(format: "%.1fk", Double(tokens) / 1_000) }
        return "\(tokens)"
    }
}

/// "resets 2h 14m" / "resets 45m" / "resets now". Compact relative future.
func resetCountdown(_ resetAtUnix: Int64) -> String {
    let remaining = Int(resetAtUnix) - Int(Date().timeIntervalSince1970)
    if remaining <= 0 { return "resets now" }
    let minutes = remaining / 60
    if minutes < 60 { return "resets \(minutes)m" }
    let hours = minutes / 60
    let mins = minutes % 60
    if hours < 24 {
        return mins == 0 ? "resets \(hours)h" : "resets \(hours)h \(mins)m"
    }
    let days = hours / 24
    return "resets \(days)d"
}
