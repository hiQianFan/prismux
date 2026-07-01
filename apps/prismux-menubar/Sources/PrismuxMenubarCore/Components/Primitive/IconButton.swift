import SwiftUI

struct IconButton: View {
    let systemName: String
    let accessibilityLabel: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: systemName)
        }
        .buttonStyle(.borderless)
        .help(accessibilityLabel)
        .accessibilityLabel(accessibilityLabel)
    }
}
