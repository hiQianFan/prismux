import SwiftUI

struct QuotaMeterProps: Equatable {
    let remainingPercentX100: UInt32?
    let label: String
}

struct MeterRing: View {
    let props: QuotaMeterProps

    var body: some View {
        Text(props.label)
            .font(.caption.monospacedDigit())
            .foregroundStyle(color)
    }

    private var color: Color {
        guard let remaining = props.remainingPercentX100 else { return .secondary }
        if remaining <= 1_500 { return .red }
        if remaining <= 4_000 { return .orange }
        return .green
    }
}
