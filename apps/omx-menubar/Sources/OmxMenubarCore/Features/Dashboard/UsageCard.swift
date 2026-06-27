import SwiftUI

/// Token-usage card with a Today / 7d / 30d toggle. The hourly buckets from the
/// backend are the single source of truth; Today renders 24 hourly bars and
/// 7d/30d roll the same series up into daily bars (see `UsageSeries`).
struct UsageCard: View {
    let usage: UsageSummary
    let title: String
    /// Today-only model breakdown, shown under the Today tab only.
    var accent: Color = .purple
    let period: UsagePeriod
    let onSelectPeriod: (UsagePeriod) -> Void

    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var buckets: [HourlyBucket] { usage.hourlyBuckets ?? [] }
    private var bars: [UsageBar] { UsageSeries.bars(from: buckets, period: period) }
    private var total: UInt64 { UsageSeries.total(bars) }

    var body: some View {
        VStack(alignment: .leading, spacing: OmxTokens.Spacing.md) {
            header

            UsageBarChart(bars: bars, accent: accent, period: period)
                .frame(height: 92)
                .animation(reduceMotion ? nil : .smooth(duration: 0.2), value: period)

            footer
        }
        .padding(OmxTokens.Spacing.md)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(cardBackground, in: RoundedRectangle(cornerRadius: OmxTokens.Radius.panel))
        .overlay(
            RoundedRectangle(cornerRadius: OmxTokens.Radius.panel)
                .stroke(Color.primary.opacity(0.08), lineWidth: 1)
        )
    }

    private var header: some View {
        HStack(alignment: .firstTextBaseline) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.subheadline.weight(.semibold))
                Text(tokenText(total) + " tokens · " + period.label)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            PeriodToggle(period: period, onSelect: onSelectPeriod)
        }
    }

    @ViewBuilder
    private var footer: some View {
        if period == .today {
            // model_breakdown is today-scoped data — only honest under Today.
            if usage.modelBreakdown.isEmpty {
                Text("No model usage today")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            } else {
                ModelLegend(models: usage.modelBreakdown, accent: accent)
            }
        } else {
            let active = bars.filter { $0.tokens > 0 }.count
            Text("\(active) of \(bars.count) days with usage")
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
    }

    private var cardBackground: Color {
        Color.primary.opacity(0.04)
    }

    private func tokenText(_ tokens: UInt64) -> String {
        if tokens >= 1_000_000 { return String(format: "%.1fM", Double(tokens) / 1_000_000) }
        if tokens >= 1_000 { return String(format: "%.1fk", Double(tokens) / 1_000) }
        return "\(tokens)"
    }
}

/// Segmented Today / 7d / 30d control. Custom (not SwiftUI Picker) so it stays
/// compact and matches the card's visual language.
private struct PeriodToggle: View {
    let period: UsagePeriod
    let onSelect: (UsagePeriod) -> Void
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        HStack(spacing: 2) {
            ForEach(UsagePeriod.allCases) { value in
                let selected = value == period
                Button {
                    if reduceMotion {
                        onSelect(value)
                    } else {
                        withAnimation(.smooth(duration: 0.22)) { onSelect(value) }
                    }
                } label: {
                    Text(value.label)
                        .font(.caption2.weight(.semibold))
                        .padding(.horizontal, 9)
                        .padding(.vertical, 4)
                        .foregroundStyle(selected ? Color.primary : .secondary)
                        .background(
                            Capsule().fill(Color.primary.opacity(selected ? 0.10 : 0))
                        )
                        .contentShape(Capsule())
                }
                .buttonStyle(.plain)
                .accessibilityAddTraits(selected ? [.isSelected] : [])
            }
        }
        .padding(2)
        .background(Color.primary.opacity(0.04), in: Capsule())
    }
}

/// The bars themselves. Today = 24 hourly bars; 7d/30d = one bar per day.
private struct UsageBarChart: View {
    let bars: [UsageBar]
    let accent: Color
    let period: UsagePeriod

    private var maxTokens: UInt64 { max(1, bars.map(\.tokens).max() ?? 1) }
    private var hasUsage: Bool { bars.contains { $0.tokens > 0 } }

    var body: some View {
        if !hasUsage {
            emptyState
        } else {
            GeometryReader { proxy in
                let spacing: CGFloat = period == .thirtyDays ? 2 : 3
                let count = bars.count
                let barWidth = max(2, (proxy.size.width - spacing * CGFloat(count - 1)) / CGFloat(count))
                HStack(alignment: .bottom, spacing: spacing) {
                    ForEach(bars) { bar in
                        barColumn(bar, width: barWidth, maxHeight: proxy.size.height)
                    }
                }
            }
        }
    }

    private func barColumn(_ bar: UsageBar, width: CGFloat, maxHeight: CGFloat) -> some View {
        let fraction = Double(bar.tokens) / Double(maxTokens)
        let barHeight = bar.tokens == 0 ? 2 : max(3, CGFloat(fraction) * (maxHeight - 14))
        return VStack(spacing: 3) {
            Spacer(minLength: 0)
            RoundedRectangle(cornerRadius: 2)
                .fill(bar.tokens == 0 ? Color.primary.opacity(0.08) : accent.opacity(bar.isCurrent ? 1 : 0.6))
                .frame(width: width, height: barHeight)
            if showLabel(bar) {
                Text(bar.label)
                    .font(.system(size: 7))
                    .foregroundStyle(bar.isCurrent ? Color.primary : .secondary)
                    .lineLimit(1)
                    .fixedSize()
            }
        }
        .frame(maxHeight: .infinity, alignment: .bottom)
        .accessibilityLabel("\(bar.label): \(bar.tokens) tokens")
    }

    /// Avoid label crowding: every bar for short series, every Nth otherwise.
    private func showLabel(_ bar: UsageBar) -> Bool {
        switch period {
        case .today:
            guard let hour = Int(bar.label) else { return false }
            return hour % 6 == 0 || bar.isCurrent
        case .sevenDays:
            return true
        case .thirtyDays:
            return bar.isCurrent
        }
    }

    private var emptyState: some View {
        Text(period == .today ? "No usage today" : "No usage in this period")
            .font(.caption)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Color.primary.opacity(0.03), in: RoundedRectangle(cornerRadius: 8))
    }
}

/// Compact color-keyed legend for today's model mix.
private struct ModelLegend: View {
    let models: [UsageModelBreakdown]
    let accent: Color

    var body: some View {
        HStack(spacing: 10) {
            ForEach(Array(models.prefix(4).enumerated()), id: \.offset) { index, model in
                HStack(spacing: 4) {
                    Circle()
                        .fill(color(for: model.model, index: index))
                        .frame(width: 6, height: 6)
                    Text(shortModel(model.model))
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }
            Spacer(minLength: 0)
        }
    }

    private func shortModel(_ model: String) -> String {
        model.replacingOccurrences(of: "claude-", with: "")
            .replacingOccurrences(of: "gpt-", with: "g")
            .replacingOccurrences(of: "gemini-", with: "gm-")
    }

    private func color(for model: String, index: Int) -> Color {
        let lower = model.lowercased()
        if lower.contains("claude") { return .orange }
        if lower.contains("gemini") { return .blue }
        if lower.contains("gpt") || lower.contains("codex") { return .green }
        return [.purple, .pink, .teal, .indigo][index % 4]
    }
}
