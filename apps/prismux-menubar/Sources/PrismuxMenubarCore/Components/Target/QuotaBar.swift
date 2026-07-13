import SwiftUI

/// Shared column widths so every quota line (5h / 7d) aligns vertically across
/// account cards and the Overview's provider cards. One set of constants instead
/// of ad-hoc per-row widths.
enum QuotaLayout {
    static let labelWidth: CGFloat = 30

    /// Below these *remaining* fractions the quota is getting tight. Apple's
    /// battery/disk convention: comfortable → yellow → red.
    static let warnThreshold = 0.50
    static let criticalThreshold = 0.20
}

/// The shared quota visual: a text row (label · percent … trailing note) over a
/// thin full-width bar with threshold ticks. Bar color encodes *health* of the
/// remaining quota (healthy → low → critical); the window is named by its label,
/// so color is free to warn. Both the account card's `QuotaLine` and the
/// Overview's provider card render through this, so they stay pixel-identical.
struct QuotaBar: View {
    let label: String
    let remainingPercentX100: UInt32?
    /// Optional trailing note (e.g. a reset time). Omitted on aggregate bars.
    var trailing: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack(spacing: 6) {
                Text(label)
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(.secondary)
                    .frame(width: QuotaLayout.labelWidth, alignment: .leading)

                Text(percentText)
                    .font(.caption2.monospacedDigit().weight(.semibold))
                    .foregroundStyle(barColor)

                Spacer(minLength: 8)

                if let trailing {
                    Text(trailing)
                        .font(.caption2.monospacedDigit())
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }

            GeometryReader { proxy in
                ZStack(alignment: .leading) {
                    Capsule().fill(Color.primary.opacity(0.12))
                    Capsule()
                        .fill(barColor)
                        .frame(width: max(3, proxy.size.width * fraction))

                    // Threshold ticks: neutral reference lines, not alarms.
                    tick(at: QuotaLayout.warnThreshold, width: proxy.size.width)
                    tick(at: QuotaLayout.criticalThreshold, width: proxy.size.width)
                }
            }
            .frame(height: 5)
        }
        .help(helpText)
        .accessibilityLabel(helpText)
    }

    private func tick(at value: Double, width: CGFloat) -> some View {
        Rectangle()
            .fill(Color.primary.opacity(0.3))
            .frame(width: 1, height: 5)
            .offset(x: width * value)
    }

    private var barColor: Color {
        guard remainingPercentX100 != nil else { return .secondary }
        if fraction <= QuotaLayout.criticalThreshold { return .red }
        if fraction <= QuotaLayout.warnThreshold { return .yellow }
        return .green
    }

    private var fraction: Double {
        guard let remaining = remainingPercentX100 else { return 0 }
        return max(0, min(1, Double(remaining) / 10_000.0))
    }

    private var percentText: String {
        guard let remaining = remainingPercentX100 else { return "--" }
        return "\(Int(remaining) / 100)%"
    }

    private var helpText: String {
        let note = trailing.map { ", \($0)" } ?? ""
        return "\(label) \(percentText)\(note)"
    }
}

/// One quota window from an account, mapped into the shared `QuotaBar`. Used by
/// account cards; the Overview's provider cards call `QuotaBar` directly with
/// the per-window-class average.
struct QuotaLine: View {
    let window: QuotaWindow
    let label: String

    var body: some View {
        QuotaBar(
            label: label,
            remainingPercentX100: window.remainingPercentX100,
            trailing: window.resetAtUnix.map(quotaResetLabel)
        )
    }
}

func quotaResetLabel(_ timestamp: Int64) -> String {
    let date = Date(timeIntervalSince1970: TimeInterval(timestamp))
    let formatter = DateFormatter()
    formatter.dateFormat = "yyyy-MM-dd HH:mm"
    return formatter.string(from: date)
}
