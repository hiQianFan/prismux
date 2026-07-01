import SwiftUI

struct DashboardScreen: View {
    @ObservedObject var store: AppStore
    var onOpenSettings: (MenubarSettingsTab) -> Void = { _ in }
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private let width: CGFloat = 392
    private let height: CGFloat = 640

    var body: some View {
        Group {
            switch store.state {
            case .loading:
                loadingView
            case .failed(let lastGood, let message), .backendUnavailable(let lastGood, let message):
                if let lastGood {
                    DashboardView(store: store, report: lastGood, stale: true, onOpenSettings: onOpenSettings)
                } else {
                    failedView(message)
                }
            case .upgradeRequired(let message):
                failedView(message)
            case .ready(let report, let stale):
                DashboardView(store: store, report: report, stale: stale, onOpenSettings: onOpenSettings)
            }
        }
        .frame(width: width, height: height)
        .background(shellBackground)
        .animation(reduceMotion ? nil : .smooth(duration: 0.18), value: store.selectedProvider)
        .animation(reduceMotion ? nil : .smooth(duration: 0.16), value: store.refreshingProvider)
        .animation(reduceMotion ? nil : .smooth(duration: 0.16), value: store.switchingLocalId)
        .animation(reduceMotion ? nil : .smooth(duration: 0.16), value: store.deletingLocalId)
        .animation(reduceMotion ? nil : .smooth(duration: 0.16), value: store.confirmingDeleteTargetId)
    }

    private var loadingView: some View {
        VStack(alignment: .leading, spacing: 14) {
            DashboardHeader(title: "Prismux", subtitle: "Loading...")
                .padding()
            VStack(spacing: 10) {
                ProgressView()
                Text("Loading dashboard")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity)
            .padding(14)
            .background(Color.primary.opacity(0.045), in: RoundedRectangle(cornerRadius: 10))
            .padding(.horizontal)
            Spacer()
        }
    }

    private func failedView(_ message: String) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            DashboardHeader(title: "Prismux", subtitle: "Backend unavailable")
                .padding()
            VStack(alignment: .leading, spacing: 10) {
                Text(message)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(4)
                Button("Retry") { Task { await store.load() } }
                    .buttonStyle(.borderless)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(14)
            .background(Color.primary.opacity(0.045), in: RoundedRectangle(cornerRadius: 10))
            .padding(.horizontal)
            Spacer()
        }
    }

    private var shellBackground: Color {
        colorScheme == .dark ? Color(red: 0.18, green: 0.16, blue: 0.34) : Color(nsColor: .windowBackgroundColor)
    }
}
