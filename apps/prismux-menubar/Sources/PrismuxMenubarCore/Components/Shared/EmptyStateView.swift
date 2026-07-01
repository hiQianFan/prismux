import SwiftUI

struct EmptyStateProps: Equatable {
    let message: String
}

struct EmptyStateView: View {
    let props: EmptyStateProps

    var body: some View {
        Text(props.message)
            .font(.caption)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, alignment: .leading)
    }
}
