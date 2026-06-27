import Foundation

/// The period selector shown on the usage card. Today renders hourly bars;
/// 7d / 30d roll the same hourly series up into daily bars.
public enum UsagePeriod: String, CaseIterable, Identifiable {
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

    /// Number of trailing days the period spans (today counts as 1).
    public var dayCount: Int {
        switch self {
        case .today: return 1
        case .sevenDays: return 7
        case .thirtyDays: return 30
        }
    }
}

/// One bar in the usage chart — an hour (Today) or a day (7d/30d).
public struct UsageBar: Identifiable, Equatable {
    /// Stable key: "YYYY-MM-DDTHH" for hours, "YYYY-MM-DD" for days.
    public let id: String
    /// Short axis label, e.g. "09" for 9am, "Mon" / "6/27" for days.
    public let label: String
    public let tokens: UInt64
    /// True for the bucket representing the current hour/day (highlight).
    public let isCurrent: Bool
}

/// Pure rollup of the backend's hourly buckets into chart bars for a period.
///
/// The backend buckets are already in the user's local timezone (the hour
/// string is computed with the local offset), so this is plain string slicing
/// — no timezone conversion. `day = prefix(10)`, `hour = suffix(2)`.
public enum UsageSeries {
    /// Build the bars for `period` from `buckets`, anchored at `now`.
    /// Missing hours/days are filled with zero so the axis is continuous.
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
                tokens: byHour[hour] ?? 0,
                isCurrent: hour == currentHour
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
                label: dayLabel(date, dayCount: dayCount, calendar: calendar),
                tokens: byDay[key] ?? 0,
                isCurrent: offset == 0
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

    private static func dayLabel(_ date: Date, dayCount: Int, calendar: Calendar) -> String {
        let c = calendar.dateComponents([.month, .day], from: date)
        guard let m = c.month, let d = c.day else { return "" }
        // 7d: short weekday is friendlier; 30d: numeric m/d to stay compact.
        if dayCount <= 7 {
            let weekday = calendar.component(.weekday, from: date)
            let symbols = calendar.veryShortWeekdaySymbols
            if weekday >= 1, weekday <= symbols.count {
                return symbols[weekday - 1]
            }
        }
        return "\(m)/\(d)"
    }
}
