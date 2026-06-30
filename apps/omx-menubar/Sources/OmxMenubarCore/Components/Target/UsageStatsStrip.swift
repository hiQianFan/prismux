import SwiftUI

/// Token usage summary strip: total + cost as paired focal points, with the
/// input/output split as a subordinate annotation underneath. Reused by both
/// the Overview (fed the all-platform headline) and a provider page (fed that
/// provider's headline) — same component, different data.
///
/// Pure presentation: reads a `UsageHeadline`, computes nothing.
struct UsageStatsStrip: View {
    let headline: UsageHeadline
    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            // Focal row: how much (tokens) and how much it cost ($), paired.
            HStack(alignment: .firstTextBaseline) {
                Text("\(tokenText(headline.totalTokens)) tokens")
                    .font(.title3.monospacedDigit().weight(.semibold))
                    .lineLimit(1)
                Spacer(minLength: 8)
                if let cost = costText {
                    Text(cost)
                        .font(.callout.monospacedDigit().weight(.semibold))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }

            // Subordinate: input/output breakdown of the total above.
            if let split = splitText {
                Text(split)
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(.secondary)
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(cardBackground, in: RoundedRectangle(cornerRadius: OmxTokens.Radius.panel))
        .overlay(
            RoundedRectangle(cornerRadius: OmxTokens.Radius.panel)
                .stroke(Color.primary.opacity(colorScheme == .dark ? 0.12 : 0.08), lineWidth: 1)
        )
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(accessibilitySummary)
    }

    /// "↓ 1.6M in   ↑ 0.8M out" — arrows mirror download/upload. Omitted when the
    /// backend didn't report the split.
    private var splitText: String? {
        guard let input = headline.inputTokens, let output = headline.outputTokens,
              input > 0 || output > 0 else { return nil }
        return "↓ \(tokenText(input)) in    ↑ \(tokenText(output)) out"
    }

    /// Cost degraded by status; Missing omits the figure entirely (never $0).
    private var costText: String? {
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
        var parts = ["\(tokenText(headline.totalTokens)) tokens"]
        if let input = headline.inputTokens, let output = headline.outputTokens, input > 0 || output > 0 {
            parts.append("\(tokenText(input)) input, \(tokenText(output)) output")
        }
        if let cost = costText { parts.append(cost) }
        return parts.joined(separator: ", ")
    }

    private var cardBackground: Color {
        colorScheme == .dark ? Color.white.opacity(0.08) : Color.white.opacity(0.86)
    }

    private func tokenText(_ tokens: UInt64) -> String {
        if tokens >= 1_000_000 { return String(format: "%.1fM", Double(tokens) / 1_000_000) }
        if tokens >= 1_000 { return String(format: "%.1fk", Double(tokens) / 1_000) }
        return "\(tokens)"
    }
}
