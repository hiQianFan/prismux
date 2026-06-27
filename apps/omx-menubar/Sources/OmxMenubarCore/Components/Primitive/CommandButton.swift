import SwiftUI

struct CommandButton: View {
    let title: String
    let systemImage: String?
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            if let systemImage {
                Label(title, systemImage: systemImage)
            } else {
                Text(title)
            }
        }
        .buttonStyle(.bordered)
        .controlSize(.small)
    }
}
