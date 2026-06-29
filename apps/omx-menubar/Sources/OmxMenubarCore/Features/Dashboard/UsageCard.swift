import Charts
import SwiftUI

/// Token-usage card with a Today / 7d / 30d toggle. The hourly buckets from the
/// backend are the single source of truth; Today renders 24 hourly bars and
/// 7d/30d roll the same series up into daily bars (see `UsageSeries`).
///
/// On the Overview the bars are stacked by provider (one color segment each);
/// on a provider page there is a single `accent`-colored series.
struct UsageCard: View {
    let usage: UsageSummary
    let title: String
    /// Today-only model breakdown, shown under the Today tab only.
    var accent: Color = .purple
    /// When non-empty, bars are stacked per provider instead of single-color.
    var providerUsage: [ProviderUsageSummary] = []
    let period: UsagePeriod
    let onSelectPeriod: (UsagePeriod) -> Void

    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var stacked: Bool { !providerUsage.isEmpty }

    private var bars: [UsageBar] {
        if stacked {
            let series = providerUsage.map {
                (provider: $0.provider, buckets: $0.usage.hourlyBuckets ?? [])
            }
            return UsageSeries.stackedBars(from: series, period: period)
        }
        return UsageSeries.bars(from: usage.hourlyBuckets ?? [], period: period)
    }

    private var total: UInt64 { UsageSeries.total(bars) }

    var body: some View {
        VStack(alignment: .leading, spacing: OmxTokens.Spacing.md) {
            header

            UsageBarChart(bars: bars, accent: accent, period: period, stacked: stacked)
                .frame(height: 96)
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
        VStack(alignment: .leading, spacing: 6) {
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
            if stacked {
                ProviderLegend(providers: providerUsage.map(\.provider))
            }
        }
    }

    @ViewBuilder
    private var footer: some View {
        // model_breakdown is today-scoped data — only honest under Today.
        // 7d/30d intentionally have no footer line; the bars and hover carry it.
        if period == .today {
            if usage.modelBreakdown.isEmpty {
                Text("No model usage today")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            } else {
                ModelLegend(models: usage.modelBreakdown, accent: accent)
            }
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

/// Stacked usage bars via Swift Charts. Today = 24 hourly bars; 7d/30d = one
/// bar per day. The x-axis only labels the first and last bucket; hovering a
/// bar reveals its time and per-provider breakdown.
private struct UsageBarChart: View {
    let bars: [UsageBar]
    let accent: Color
    let period: UsagePeriod
    let stacked: Bool

    @State private var selectedId: String?

    private var hasUsage: Bool { bars.contains { $0.tokens > 0 } }

    private var selectedBar: UsageBar? {
        selectedId.flatMap { id in bars.first { $0.id == id } }
    }

    var body: some View {
        if !hasUsage {
            emptyState
        } else {
            VStack(spacing: 4) {
                chart
                axisFooter
            }
        }
    }

    /// Head/tail time labels under the chart, left/right aligned so neither
    /// clips at the plot edge (an in-chart AxisMark centers on the edge bar and
    /// gets truncated). The middle is left to hover.
    private var axisFooter: some View {
        HStack {
            Text(bars.first?.label ?? "")
            Spacer()
            Text(bars.last?.label ?? "")
        }
        .font(.system(size: 9))
        .foregroundStyle(.secondary)
    }

    private var chart: some View {
        Chart {
            ForEach(bars) { bar in
                // Faint placeholder track for every slot — keeps empty hours/days
                // visible so the time axis reads as continuous instead of
                // collapsing the gaps.
                if bar.tokens == 0 {
                    BarMark(
                        x: .value("Time", bar.id),
                        y: .value("Tokens", emptyTrackTokens),
                        width: barWidth
                    )
                    .foregroundStyle(Color.primary.opacity(0.06))
                    .cornerRadius(2)
                } else if stacked {
                    ForEach(bar.segments.filter { $0.tokens > 0 }) { segment in
                        BarMark(
                            x: .value("Time", bar.id),
                            y: .value("Tokens", segment.tokens),
                            width: barWidth
                        )
                        .foregroundStyle(ProviderStyle.color(segment.provider))
                        .cornerRadius(2)
                    }
                } else {
                    BarMark(
                        x: .value("Time", bar.id),
                        y: .value("Tokens", bar.tokens),
                        width: barWidth
                    )
                    .foregroundStyle(accent.opacity(bar.isCurrent ? 1 : 0.7))
                    .cornerRadius(2)
                }
            }

            // Selection: a faint rule anchors the tooltip to the hovered bar.
            // `overflowResolution` keeps the popup inside the chart instead of
            // pinning it center-top, so it follows the bar and never clips.
            if let bar = selectedBar {
                RuleMark(x: .value("Time", bar.id))
                    .foregroundStyle(Color.primary.opacity(0.12))
                    .lineStyle(StrokeStyle(lineWidth: 1))
                    .annotation(
                        position: .top,
                        spacing: 2,
                        overflowResolution: .init(x: .fit(to: .chart), y: .fit(to: .chart))
                    ) {
                        HoverTooltip(bar: bar, period: period, stacked: stacked)
                    }
            }
        }
        // Headroom above the tallest bar so the .top annotation has room to sit
        // without covering bars.
        .chartYScale(domain: 0...Double(max(1, bars.map(\.tokens).max() ?? 1)) * 1.25)
        .chartXSelection(value: $selectedId)
        .chartXAxis(.hidden)
        .chartYAxis(.hidden)
        .chartLegend(.hidden)
        .accessibilityLabel(Text("\(period.label) token usage"))
    }

    /// Ratio width leaves an even gap between bars at any count, so Today (24),
    /// 7d (7) and 30d (30) all read with consistent spacing instead of fusing
    /// into a solid block or floating far apart.
    private var barWidth: MarkDimension {
        .ratio(0.6)
    }

    /// Tiny stub height for empty slots — visible as a track, not as usage.
    private var emptyTrackTokens: Double {
        Double(max(1, bars.map(\.tokens).max() ?? 1)) * 0.02
    }

    private var emptyState: some View {
        Text(period == .today ? "No usage today" : "No usage in this period")
            .font(.caption)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Color.primary.opacity(0.03), in: RoundedRectangle(cornerRadius: 8))
    }
}

/// Floating readout for the hovered bar: its time and per-provider tokens.
private struct HoverTooltip: View {
    let bar: UsageBar
    let period: UsagePeriod
    let stacked: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(timeLabel)
                .font(.caption2.weight(.semibold))
            if stacked {
                ForEach(bar.segments.filter { $0.tokens > 0 }) { segment in
                    HStack(spacing: 4) {
                        Circle()
                            .fill(ProviderStyle.color(segment.provider))
                            .frame(width: 6, height: 6)
                        Text("\(segment.provider.capitalized) \(tokenText(segment.tokens))")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                }
            }
            Text("Total \(tokenText(bar.tokens))")
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 6))
        .overlay(
            RoundedRectangle(cornerRadius: 6).stroke(Color.primary.opacity(0.1), lineWidth: 1)
        )
        .fixedSize()
    }

    private var timeLabel: String {
        // Today's bar.label is "HH"; days already carry a readable date label.
        period == .today ? "\(bar.label):00" : bar.fullLabel
    }

    private func tokenText(_ tokens: UInt64) -> String {
        if tokens >= 1_000_000 { return String(format: "%.1fM", Double(tokens) / 1_000_000) }
        if tokens >= 1_000 { return String(format: "%.1fk", Double(tokens) / 1_000) }
        return "\(tokens)"
    }
}

/// Provider color key shown above the stacked Overview chart.
private struct ProviderLegend: View {
    let providers: [String]

    var body: some View {
        HStack(spacing: 10) {
            ForEach(providers, id: \.self) { provider in
                HStack(spacing: 4) {
                    Circle()
                        .fill(ProviderStyle.color(provider))
                        .frame(width: 6, height: 6)
                    Text(provider.capitalized)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }
            Spacer(minLength: 0)
        }
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
