import SwiftUI

struct StatusBannerProps: Equatable {
    let severity: Severity
    let title: String
    let message: String

    enum Severity: Equatable {
        case info
        case warning
        case error
    }
}

struct StatusBanner: View {
    let props: StatusBannerProps

    var body: some View {
        HStack(alignment: .top, spacing: PrismuxTokens.Spacing.sm) {
            Image(systemName: icon)
                .foregroundStyle(color)
            VStack(alignment: .leading, spacing: 2) {
                Text(props.title)
                    .font(.caption.weight(.semibold))
                Text(props.message)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            Spacer()
        }
        .padding(PrismuxTokens.Spacing.md)
        .background(color.opacity(0.12), in: RoundedRectangle(cornerRadius: PrismuxTokens.Radius.panel))
    }

    private var icon: String {
        switch props.severity {
        case .info: return "info.circle"
        case .warning: return "exclamationmark.triangle"
        case .error: return "xmark.octagon"
        }
    }

    private var color: Color {
        switch props.severity {
        case .info: return .blue
        case .warning: return PrismuxTokens.StatusColor.warning
        case .error: return PrismuxTokens.StatusColor.failed
        }
    }
}
