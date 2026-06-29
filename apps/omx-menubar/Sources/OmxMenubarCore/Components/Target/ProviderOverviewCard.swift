import SwiftUI

/// Provider-page Overview, organized around the page's one decision: who's
/// active, who can I switch to, and when does pressure ease. Replaces the old
/// four bare MetricCells (Tokens / Targets / Lowest / Alerts) whose data already
/// lived in the Accounts card header and Diagnostics.
///
/// Everything is read from the backend aggregate; the view computes no
/// thresholds and no aggregation. Same average headroom figure as the global
/// Overview, then the drill-down (best alternative, reset credits) that only
/// makes sense on the provider page.
struct ProviderOverviewCard: View {
    let provider: String
    let aggregate: ProviderAggregateView?
    let activeLabel: String?
    let accent: Color

    @Environment(\.colorScheme) private var colorScheme

    private var health: QuotaHealthRollup? { aggregate?.quotaHealth }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("\(provider.capitalized) Overview")
                .font(.headline)

            activeRow
            Divider().opacity(0.4)
            breakdownRow
            avgRow
            if let alternative = bestAlternative {
                Divider().opacity(0.4)
                alternativeRow(alternative)
            }
            if let credits = resetCreditTotal, credits > 0 {
                resetCreditRow(credits)
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(cardBackground, in: RoundedRectangle(cornerRadius: OmxTokens.Radius.panel))
        .overlay(
            RoundedRectangle(cornerRadius: OmxTokens.Radius.panel)
                .stroke(Color.primary.opacity(colorScheme == .dark ? 0.12 : 0.08), lineWidth: 1)
        )
    }

    // MARK: Active

    private var activeRow: some View {
        HStack(spacing: 8) {
            Circle().fill(accent).frame(width: 8, height: 8).accessibilityHidden(true)
            Text("Active")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
            Spacer(minLength: 8)
            Text(activeText)
                .font(.callout.weight(.semibold).monospacedDigit())
                .lineLimit(1)
                .truncationMode(.middle)
        }
    }

    // MARK: Capacity breakdown

    private var breakdownRow: some View {
        HStack {
            Text("Accounts")
                .font(.caption)
                .foregroundStyle(.secondary)
            Spacer()
            Text(breakdownText)
                .font(.caption.monospacedDigit())
                .foregroundStyle(.secondary)
        }
    }

    private var avgRow: some View {
        HStack(spacing: 8) {
            Text("Avg remaining")
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(avgText)
                .font(.caption.monospacedDigit().weight(.semibold))
                .foregroundStyle(toneColor)
            if let fraction = avgFraction {
                GeometryReader { proxy in
                    ZStack(alignment: .leading) {
                        Capsule().fill(Color.primary.opacity(0.12))
                        Capsule().fill(toneColor)
                            .frame(width: max(3, proxy.size.width * fraction))
                    }
                }
                .frame(height: 4)
            }
        }
    }

    // MARK: Best alternative

    private func alternativeRow(_ alternative: TargetRecommendation) -> some View {
        HStack(spacing: 6) {
            Image(systemName: "arrowshape.turn.up.right")
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.tertiary)
            Text("Best alternative")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
            Spacer(minLength: 8)
            Text(alternative.target.displayLabel)
                .font(.caption.monospacedDigit())
                .lineLimit(1)
                .truncationMode(.middle)
        }
    }

    private func resetCreditRow(_ credits: UInt32) -> some View {
        HStack {
            Text("Reset credits")
                .font(.caption)
                .foregroundStyle(.secondary)
            Spacer()
            Text("\(credits) available")
                .font(.caption.monospacedDigit())
                .foregroundStyle(.secondary)
        }
    }

    // MARK: Derived

    private var activeText: String {
        guard let activeLabel, !activeLabel.isEmpty, activeLabel != "-" else { return "none" }
        return activeLabel
    }

    private var breakdownText: String {
        guard let health else { return "—" }
        let healthy = Int(health.healthyCount)
        let low = Int(health.lowCount)
        let exhausted = Int(health.exhaustedCount)
        let total = Int(health.facts.accountCount)
        if total == 0 { return "no accounts" }
        var parts = ["\(total) account\(total == 1 ? "" : "s")"]
        if healthy > 0 { parts.append("\(healthy) healthy") }
        if low > 0 { parts.append("\(low) low") }
        if exhausted > 0 { parts.append("\(exhausted) exhausted") }
        return parts.joined(separator: " · ")
    }

    private var avgText: String {
        guard let avg = health?.facts.avgRemainingPercentX100 else { return "—" }
        return "\(Int(avg) / 100)%"
    }

    private var avgFraction: Double? {
        guard let avg = health?.facts.avgRemainingPercentX100 else { return nil }
        return max(0, min(1, Double(avg) / 10_000.0))
    }

    private var toneColor: Color {
        switch health?.statusTone {
        case "success": return OmxTokens.StatusColor.healthy
        case "warning": return OmxTokens.StatusColor.warning
        case "danger": return OmxTokens.StatusColor.failed
        default: return OmxTokens.StatusColor.muted
        }
    }

    private var bestAlternative: TargetRecommendation? { health?.bestAlternative }
    private var resetCreditTotal: UInt32? { health?.facts.resetCreditTotal }

    private var cardBackground: Color {
        colorScheme == .dark ? Color.white.opacity(0.08) : Color.white.opacity(0.86)
    }
}
