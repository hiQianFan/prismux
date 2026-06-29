import Charts
import SwiftUI

/// Token-usage card with a Today / 7d / 30d toggle. The hourly buckets from the
/// backend are the single source of truth; Today renders 24 hourly bars and
/// 7d/30d roll the same series up into daily bars (see `UsageSeries`).
///
/// The backend decides the segment dimension: Overview uses providers, and a
/// provider page uses models.
struct UsageCard: View {
    let usage: UsageSummary
    let title: String
    var accent: Color = .purple
    var themeProvider: String?
    let period: UsagePeriod
    let onSelectPeriod: (UsagePeriod) -> Void

    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var series: [UsageChartSeries] { usage.series ?? [] }
    private var stacksBars: Bool { themeProvider == nil && !series.isEmpty }
    private var showsComposition: Bool { !series.isEmpty }

    private var rawBars: [UsageBar] {
        if stacksBars {
            return UsageSeries.stackedBars(from: series, period: period)
        }
        return UsageSeries.bars(from: usage.hourlyBuckets ?? [], period: period)
    }

    private var bars: [UsageBar] {
        stacksBars ? UsageSeriesRanker.rankedBars(rawBars) : rawBars
    }

    private var compositionBars: [UsageBar] {
        UsageSeriesRanker.rankedBars(UsageSeries.stackedBars(from: series, period: period))
    }

    private var legendItems: [UsageLegendItem] {
        UsageSeriesRanker.legendItems(compositionBars)
    }

    private var total: UInt64 { UsageSeries.total(bars) }

    var body: some View {
        VStack(alignment: .leading, spacing: OmxTokens.Spacing.md) {
            header

            UsageBarChart(
                bars: bars,
                accent: accent,
                themeProvider: themeProvider,
                period: period,
                stacked: stacksBars
            )
                .frame(height: 96)
                .animation(reduceMotion ? nil : .smooth(duration: 0.2), value: period)
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
            if showsComposition {
                UsageLegend(
                    items: legendItems,
                    showsPercent: !stacksBars,
                    themeProvider: themeProvider
                )
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
/// bar reveals its time and per-series breakdown.
private struct UsageBarChart: View {
    let bars: [UsageBar]
    let accent: Color
    let themeProvider: String?
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
                        .foregroundStyle(UsageSeriesStyle.color(segment, themeProvider: themeProvider))
                        .cornerRadius(2)
                    }
                } else {
                    BarMark(
                        x: .value("Time", bar.id),
                        y: .value("Tokens", bar.tokens),
                        width: barWidth
                    )
                    .foregroundStyle(accent.opacity(isHighlighted(bar) ? 1 : 0.7))
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
                        HoverTooltip(
                            bar: bar,
                            themeProvider: themeProvider,
                            period: period,
                            stacked: stacked
                        )
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

    private func isHighlighted(_ bar: UsageBar) -> Bool {
        if let selectedId {
            return bar.id == selectedId
        }
        return bar.isCurrent
    }
}

/// Floating readout for the hovered bar: its time and per-series tokens.
private struct HoverTooltip: View {
    let bar: UsageBar
    let themeProvider: String?
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
                            .fill(UsageSeriesStyle.color(segment, themeProvider: themeProvider))
                            .frame(width: 6, height: 6)
                        Text("\(segment.label) \(tokenText(segment.tokens))")
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

/// Color key shown above a stacked chart.
private struct UsageLegend: View {
    let items: [UsageLegendItem]
    let showsPercent: Bool
    let themeProvider: String?

    var body: some View {
        HStack(spacing: 10) {
            ForEach(items) { item in
                HStack(spacing: 4) {
                    Circle()
                        .fill(UsageSeriesStyle.color(item.segment, themeProvider: themeProvider))
                        .frame(width: 6, height: 6)
                    Text(label(for: item))
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
            }
            Spacer(minLength: 0)
        }
    }

    private func label(for item: UsageLegendItem) -> String {
        if showsPercent {
            return "\(item.segment.label) \(percentText(item.share))"
        }
        return item.segment.label
    }

    private func percentText(_ share: Double) -> String {
        if share <= 0 { return "0%" }
        if share >= 0.995 { return "100%" }
        if share >= 0.10 { return "\(Int((share * 100).rounded()))%" }
        if share < 0.001 { return "<0.1%" }
        return String(format: "%.1f%%", share * 100)
    }
}

private struct UsageLegendItem: Identifiable {
    let segment: UsageSegment
    let tokens: UInt64
    let totalTokens: UInt64

    var id: String { segment.id }
    var share: Double {
        guard totalTokens > 0 else { return 0 }
        return Double(tokens) / Double(totalTokens)
    }
}

private enum UsageSeriesRanker {
    private static let maxVisibleSegments = 5
    private static let otherKey = "__other__"

    static func rankedBars(_ bars: [UsageBar]) -> [UsageBar] {
        let ranked = rankedSegmentIds(bars)
        let visible = Set(ranked.prefix(maxVisibleSegments).map(\.id))
        let rankById = Dictionary(uniqueKeysWithValues: ranked.enumerated().map { index, entry in
            (entry.id, index)
        })
        let hasOther = ranked.count > maxVisibleSegments

        return bars.map { bar in
            var otherTokens: UInt64 = 0
            var segments = bar.segments.compactMap { segment -> UsageSegment? in
                if visible.contains(segment.id) {
                    return UsageSegment(
                        kind: segment.kind,
                        key: segment.key,
                        label: segment.label,
                        tokens: segment.tokens,
                        rank: rankById[segment.id] ?? 0
                    )
                }
                otherTokens += segment.tokens
                return nil
            }
            if hasOther {
                segments.append(UsageSegment(
                    kind: "other",
                    key: otherKey,
                    label: "Other",
                    tokens: otherTokens,
                    rank: maxVisibleSegments
                ))
            }
            return UsageBar(
                id: bar.id,
                label: bar.label,
                fullLabel: bar.fullLabel,
                tokens: bar.tokens,
                isCurrent: bar.isCurrent,
                segments: segments
            )
        }
    }

    static func legendItems(_ bars: [UsageBar]) -> [UsageLegendItem] {
        var totals: [String: (segment: UsageSegment, tokens: UInt64)] = [:]
        for segment in bars.flatMap(\.segments) {
            let current = totals[segment.id]?.tokens ?? 0
            totals[segment.id] = (segment, current + segment.tokens)
        }
        let totalTokens = totals.values.reduce(UInt64(0)) { $0 + $1.tokens }
        return totals.values
            .filter { $0.tokens > 0 }
            .sorted {
                if $0.segment.rank == $1.segment.rank {
                    return $0.segment.label < $1.segment.label
                }
                return $0.segment.rank < $1.segment.rank
            }
            .map { UsageLegendItem(segment: $0.segment, tokens: $0.tokens, totalTokens: totalTokens) }
    }

    private static func rankedSegmentIds(_ bars: [UsageBar]) -> [(id: String, label: String)] {
        var totals: [String: (label: String, tokens: UInt64)] = [:]
        for segment in bars.flatMap(\.segments) {
            let current = totals[segment.id]?.tokens ?? 0
            totals[segment.id] = (segment.label, current + segment.tokens)
        }
        return totals
            .filter { $0.value.tokens > 0 }
            .sorted {
                if $0.value.tokens == $1.value.tokens {
                    return $0.value.label < $1.value.label
                }
                return $0.value.tokens > $1.value.tokens
            }
            .map { (id: $0.key, label: $0.value.label) }
    }
}

private enum UsageSeriesStyle {
    static func color(_ segment: UsageSegment, themeProvider: String?) -> Color {
        if segment.kind == "provider" {
            return ProviderStyle.color(segment.key)
        }
        return derivedColor(base: ProviderStyle.hsb(themeProvider), rank: segment.rank)
    }

    private static func derivedColor(base: ProviderStyle.HSB, rank: Int) -> Color {
        if rank == 0 {
            return Color(hue: base.hue, saturation: base.saturation, brightness: base.brightness)
        }
        if rank >= 5 {
            return Color(
                hue: base.hue,
                saturation: max(0.18, base.saturation * 0.28),
                brightness: min(0.88, max(0.42, base.brightness * 0.92))
            )
        }
        let variants: [(hue: Double, saturation: Double, brightness: Double)] = [
            (0.060, 0.72, 1.24),
            (-0.070, 0.66, 0.74),
            (0.120, 0.58, 1.36),
            (-0.120, 0.52, 0.62),
        ]
        let variant = variants[(rank - 1) % variants.count]
        let hue = normalizedHue(base.hue + variant.hue)
        let saturation = min(0.95, max(0.30, base.saturation * variant.saturation))
        let brightness = min(0.96, max(0.36, base.brightness * variant.brightness))
        return Color(hue: hue, saturation: saturation, brightness: brightness)
    }

    private static func normalizedHue(_ hue: Double) -> Double {
        let value = hue.truncatingRemainder(dividingBy: 1)
        return value < 0 ? value + 1 : value
    }
}
