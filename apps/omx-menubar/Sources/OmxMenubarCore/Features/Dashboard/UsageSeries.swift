import Foundation

/// The period selector shown on the usage card. Today renders hourly bars;
/// 7d / 30d roll the same hourly series up into daily bars.
public enum UsagePeriod: String, CaseIterable, Identifiable, Sendable, Decodable {
    case today
    case sevenDays
    case thirtyDays

    public var id: String { rawValue }

    public var label: String {
        switch self {
        case .today: return "Today"
        case .sevenDays: return "7d"
        case .thirtyDays: return "30d"
        }
    }

    public var backendValue: String {
        switch self {
        case .today: return "Today"
        case .sevenDays: return "SevenDays"
        case .thirtyDays: return "ThirtyDays"
        }
    }

    public init(from decoder: Decoder) throws {
        let value = try decoder.singleValueContainer().decode(String.self)
        switch value {
        case "Today", "today":
            self = .today
        case "SevenDays", "sevenDays":
            self = .sevenDays
        case "ThirtyDays", "thirtyDays":
            self = .thirtyDays
        default:
            throw DecodingError.dataCorrupted(
                .init(codingPath: decoder.codingPath, debugDescription: "Unknown usage period \(value)")
            )
        }
    }

    /// Number of trailing days the period spans (today counts as 1).
    public var dayCount: Int {
        switch self {
        case .today: return 1
        case .sevenDays: return 7
        case .thirtyDays: return 30
        }
    }
}

/// One series slice of a stacked bar.
public struct UsageSegment: Identifiable, Equatable {
    public let kind: String
    public let key: String
    public let label: String
    public let tokens: UInt64
    public let rank: Int
    public var id: String { "\(kind):\(key)" }
}

/// One bar in the usage chart — an hour (Today) or a day (7d/30d).
public struct UsageBar: Identifiable, Equatable {
    /// Stable key: "YYYY-MM-DDTHH" for hours, "YYYY-MM-DD" for days.
    public let id: String
    /// Short axis label for the head/tail marks: "09" for 9am, "6/27" for days.
    public let label: String
    /// Richer label for the hover tooltip: "09:00" handled by the view for
    /// hours, "Sat 6/27" for days.
    public let fullLabel: String
    public let tokens: UInt64
    /// True for the bucket representing the current hour/day (highlight).
    public let isCurrent: Bool
    /// Per-series breakdown for stacked bars. Empty for single-series bars,
    /// where the whole bar is one accent color.
    public let segments: [UsageSegment]
}

/// Pure rollup of the backend's hourly buckets into chart bars for a period.
///
/// The backend buckets are already in the user's local timezone (the hour
/// string is computed with the local offset), so this is plain string slicing
/// — no timezone conversion. `day = prefix(10)`, `hour = suffix(2)`.
public enum UsageSeries {
    /// Build the bars for `period` from `buckets`, anchored at `now`.
    /// Missing hours/days are filled with zero so the axis is continuous.
    /// Single series — no per-provider segments (used by the provider page).
    public static func bars(
        from buckets: [HourlyBucket],
        period: UsagePeriod,
        now: Date = Date(),
        calendar: Calendar = .current
    ) -> [UsageBar] {
        switch period {
        case .today:
            return hourlyBars(from: buckets, now: now, calendar: calendar)
        case .sevenDays, .thirtyDays:
            return dailyBars(from: buckets, dayCount: period.dayCount, now: now, calendar: calendar)
        }
    }

    /// Build stacked bars from several series buckets. Each bar's `tokens`
    /// is the sum of its `segments`. Reuses the single-series rollup per
    /// series, then aligns them on the shared bar skeleton (same ids).
    public static func stackedBars(
        from series: [UsageChartSeries],
        period: UsagePeriod,
        now: Date = Date(),
        calendar: Calendar = .current
    ) -> [UsageBar] {
        let combined = series.flatMap(\.hourlyBuckets)
        let base = bars(from: combined, period: period, now: now, calendar: calendar)

        // series → (bar id → tokens)
        let perSeries: [(series: UsageChartSeries, byId: [String: UInt64])] = series.map { entry in
            let bars = bars(from: entry.hourlyBuckets, period: period, now: now, calendar: calendar)
            return (entry, Dictionary(uniqueKeysWithValues: bars.map { ($0.id, $0.tokens) }))
        }

        return base.map { bar in
            let segments = perSeries.map { entry in
                UsageSegment(
                    kind: entry.series.kind,
                    key: entry.series.key,
                    label: entry.series.label,
                    tokens: entry.byId[bar.id] ?? 0,
                    rank: 0
                )
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

    /// Total tokens across the period (sum of the bars).
    public static func total(_ bars: [UsageBar]) -> UInt64 {
        bars.reduce(UInt64(0)) { $0 + $1.tokens }
    }

    // MARK: - Today: 24 hourly bars (00..23)

    private static func hourlyBars(
        from buckets: [HourlyBucket],
        now: Date,
        calendar: Calendar
    ) -> [UsageBar] {
        let todayKey = dayKey(now, calendar: calendar)
        let currentHour = calendar.component(.hour, from: now)

        // tokens by hour-of-day for today only
        var byHour: [Int: UInt64] = [:]
        for bucket in buckets where bucket.localHour.hasPrefix(todayKey) {
            guard let hour = hourComponent(bucket.localHour) else { continue }
            byHour[hour, default: 0] += bucket.totalTokens
        }

        return (0..<24).map { hour in
            UsageBar(
                id: "\(todayKey)T\(twoDigits(hour))",
                label: twoDigits(hour),
                fullLabel: "\(twoDigits(hour)):00",
                tokens: byHour[hour] ?? 0,
                isCurrent: hour == currentHour,
                segments: []
            )
        }
    }

    // MARK: - 7d / 30d: one bar per day

    private static func dailyBars(
        from buckets: [HourlyBucket],
        dayCount: Int,
        now: Date,
        calendar: Calendar
    ) -> [UsageBar] {
        // Sum hours into days first.
        var byDay: [String: UInt64] = [:]
        for bucket in buckets {
            let day = String(bucket.localHour.prefix(10))
            byDay[day, default: 0] += bucket.totalTokens
        }

        let todayStart = calendar.startOfDay(for: now)
        // Oldest first so the chart reads left → right, past → present.
        return (0..<dayCount).reversed().compactMap { offset -> UsageBar? in
            guard let date = calendar.date(byAdding: .day, value: -offset, to: todayStart) else {
                return nil
            }
            let key = dayKey(date, calendar: calendar)
            return UsageBar(
                id: key,
                label: dayLabel(date, calendar: calendar),
                fullLabel: dayFullLabel(date, calendar: calendar),
                tokens: byDay[key] ?? 0,
                isCurrent: offset == 0,
                segments: []
            )
        }
    }

    // MARK: - Helpers

    private static func hourComponent(_ localHour: String) -> Int? {
        // "YYYY-MM-DDTHH" → HH
        guard localHour.count >= 13 else { return nil }
        return Int(localHour.suffix(2))
    }

    private static func twoDigits(_ value: Int) -> String {
        value < 10 ? "0\(value)" : "\(value)"
    }

    private static func dayKey(_ date: Date, calendar: Calendar) -> String {
        let c = calendar.dateComponents([.year, .month, .day], from: date)
        guard let y = c.year, let m = c.month, let d = c.day else { return "" }
        return "\(y)-\(twoDigits(m))-\(twoDigits(d))"
    }

    /// Axis label for a day's head/tail mark — always a concrete date ("6/27").
    /// Weekday symbols ("1,2,3,4") carry no date and were the old bug.
    private static func dayLabel(_ date: Date, calendar: Calendar) -> String {
        let c = calendar.dateComponents([.month, .day], from: date)
        guard let m = c.month, let d = c.day else { return "" }
        return "\(m)/\(d)"
    }

    /// Tooltip label: weekday + date, e.g. "Sat 6/27".
    private static func dayFullLabel(_ date: Date, calendar: Calendar) -> String {
        let weekday = calendar.component(.weekday, from: date)
        let symbols = calendar.shortWeekdaySymbols
        let day = dayLabel(date, calendar: calendar)
        if weekday >= 1, weekday <= symbols.count {
            return "\(symbols[weekday - 1]) \(day)"
        }
        return day
    }
}
