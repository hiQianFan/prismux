import AppKit
import SwiftUI

struct DashboardView: View {
    @ObservedObject var store: AppStore
    let report: DashboardReport
    let stale: Bool
    let onOpenSettings: (MenubarSettingsTab) -> Void

    @AppStorage("dev.openmux.menubar.hidePersonalIdentifiers") private var hidePersonalIdentifiers = false
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        content(report, stale: stale)
    }

    private func content(_ report: DashboardReport, stale: Bool) -> some View {
        let providers = providerNames(report)
        let currentProvider = store.selectedProvider.flatMap { providers.contains($0) ? $0 : nil }
        // Carousel index: 0 = Overview, n = providers[n-1]. One source of truth
        // for the slide offset so day/provider transitions are directional.
        let pages: [String?] = [nil] + providers.map { Optional($0) }
        let selectedIndex = currentProvider.flatMap { pages.firstIndex(of: $0) } ?? 0

        return VStack(alignment: .leading, spacing: 0) {
            header(report, stale: stale)
            ProviderTabBar(
                providers: providers,
                selected: currentProvider,
                onSelect: { provider in store.selectedProvider = provider }
            )
            .padding(.horizontal)
            .padding(.bottom, 10)

            Divider()

            if let notice = store.operationNotice {
                StatusBanner(props: StatusBannerProps(
                    severity: notice.severity,
                    title: notice.title,
                    message: notice.message
                ))
                .padding(.horizontal)
                .padding(.top, 10)
            }

            carousel(pages: pages, selectedIndex: selectedIndex, report: report)

            Divider()
            footer(providers: providers, selectedProvider: currentProvider)
        }
    }

    /// Horizontal carousel of pages, offset by the selected index. Each page
    /// scrolls independently. The slide direction falls out of the index delta
    /// for free — no transition-direction bookkeeping.
    @ViewBuilder
    private func carousel(pages: [String?], selectedIndex: Int, report: DashboardReport) -> some View {
        GeometryReader { proxy in
            let pageWidth = proxy.size.width
            HStack(spacing: 0) {
                ForEach(Array(pages.enumerated()), id: \.offset) { _, provider in
                    ScrollView {
                        VStack(alignment: .leading, spacing: 12) {
                            page(for: provider, report: report)
                        }
                        .padding()
                    }
                    .frame(width: pageWidth)
                }
            }
            .frame(width: pageWidth, alignment: .leading)
            .offset(x: -CGFloat(selectedIndex) * pageWidth)
            .animation(reduceMotion ? nil : .smooth(duration: 0.28), value: selectedIndex)
            .clipped()
        }
    }


    private func header(_ report: DashboardReport, stale: Bool) -> some View {
        HStack(alignment: .center, spacing: 12) {
            headerIcon(provider: report.active?.provider ?? providerNames(report).first ?? "openmux", stale: stale)

            VStack(alignment: .leading, spacing: 4) {
                Text("OpenMux")
                    .font(.title3.bold())
                Text(headerSubtitle(report, stale: stale))
                    .font(.caption)
                    .foregroundStyle(stale ? .orange : .secondary)
                    .lineLimit(1)
            }

            Spacer()

            HStack(spacing: 8) {
                Button {
                    Task {
                        let providers = providerNames(report)
                        if let selectedProvider = store.selectedProvider, providers.contains(selectedProvider) {
                            await store.refresh(provider: selectedProvider, kind: "interactive")
                        } else {
                            await store.refreshAll(providers: providers, kind: "interactive")
                        }
                    }
                } label: {
                    if store.refreshingProvider != nil {
                        ProgressView()
                            .controlSize(.small)
                            .frame(width: 16, height: 16)
                    } else {
                        Image(systemName: "arrow.clockwise")
                    }
                }
                .buttonStyle(IconFeedbackButtonStyle())
                .disabled(store.refreshingProvider != nil)
                .help("Refresh status")
                .accessibilityLabel("Refresh status")

                Button {
                    onOpenSettings(.general)
                } label: {
                    Image(systemName: "gearshape")
                }
                .buttonStyle(IconFeedbackButtonStyle())
                .help("Settings")
                .accessibilityLabel("Settings")
            }
        }
        .padding()
    }

    private func headerIcon(provider: String, stale: Bool) -> some View {
        ZStack {
            RoundedRectangle(cornerRadius: 10)
                .fill(headerIconBackground)
                .frame(width: 36, height: 36)
            Image(systemName: stale ? "exclamationmark.arrow.triangle.2.circlepath" : "switch.2")
                .font(.system(size: 18, weight: .semibold))
                .foregroundStyle(stale ? .orange : providerColor(provider))
        }
    }

    @ViewBuilder
    private func page(for provider: String?, report: DashboardReport) -> some View {
        if let provider {
            ProviderPage(provider: provider) {
                providerPage(provider: provider, report: report)
            }
        } else {
            OverviewPage {
                overview(report)
            }
        }
    }

    private func overview(_ report: DashboardReport) -> some View {
        let providers = providerNames(report)
        return VStack(alignment: .leading, spacing: 12) {
            Card(title: "Providers") {
                if providers.isEmpty {
                    emptyState("No providers reported by backend.")
                }
                ForEach(providers, id: \.self) { provider in
                    let accounts = accounts(for: provider, in: report)
                    let profiles = profiles(for: provider, in: report)
                    let view = providerView(provider, in: report)
                    let activeLabel = activeTargetLabel(accounts: accounts, profiles: profiles)
                    OverviewProviderRow(
                        provider: provider,
                        activeTarget: activeLabel == "-" ? nil : activeLabel,
                        accountCount: accounts.count,
                        profileCount: profiles.count,
                        lowestQuotaPercent: lowestQuota(accounts).map { Int($0) / 100 },
                        statusText: view?.statusText ?? "Unknown",
                        statusTone: view.map { toneColor($0.statusTone) } ?? .secondary,
                        onTap: { store.selectedProvider = provider }
                    )
                    if provider != providers.last {
                        Divider().opacity(0.4)
                    }
                }
            }

            UsageCard(
                usage: report.usage,
                title: "Token Usage",
                period: store.usagePeriod,
                onSelectPeriod: { store.usagePeriod = $0 }
            )
        }
    }

    private func providerPage(provider: String, report: DashboardReport) -> some View {
        let accounts = accounts(for: provider, in: report)
        let profiles = profiles(for: provider, in: report)
        let active = activeAccount(accounts)
        let activeProfile = activeProfile(profiles)

        let usage = providerUsage(provider, in: report)

        return VStack(alignment: .leading, spacing: 12) {
            providerOverview(provider: provider, accounts: accounts, profiles: profiles, active: active, activeProfile: activeProfile, usage: usage, report: report)
            accountTargets(provider: provider, accounts: accounts, report: report)
            profileTargets(provider: provider, profiles: profiles, report: report)
            UsageCard(
                usage: usage,
                title: "\(provider.capitalized) Token Usage",
                accent: ProviderStyle.color(provider),
                themeProvider: provider,
                period: store.usagePeriod,
                onSelectPeriod: { store.usagePeriod = $0 }
            )
            diagnostics(provider: provider, report: report, accounts: accounts)
        }
    }

    private func providerOverview(provider: String, accounts: [MenubarAccount], profiles: [MenubarProfile], active: MenubarAccount?, activeProfile: MenubarProfile?, usage: UsageSummary, report: DashboardReport) -> some View {
        let targetLabel = activeTargetLabel(account: active, profile: activeProfile)
        let lowest = lowestQuota(accounts).map { "\(Int($0) / 100)%" } ?? "unknown"
        let alerts = providerView(provider, in: report).map { isProviderAttention($0) ? 1 : 0 } ?? 0

        return Card(title: "\(provider.capitalized) Overview") {
            HStack(spacing: 8) {
                MetricCell(label: "Tokens", value: "\(usage.totalTokens)")
                MetricCell(label: "Targets", value: "\(accounts.count + profiles.count)")
                MetricCell(label: "Lowest", value: lowest)
                MetricCell(label: "Alerts", value: "\(alerts)")
            }

            HStack(spacing: 8) {
                providerDot(provider)
                Text("Active target")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                Spacer()
                Text(targetLabel ?? "none")
                    .font(.caption.monospacedDigit())
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
        }
    }

    private func accountTargets(provider: String, accounts: [MenubarAccount], report: DashboardReport) -> some View {
        Card(title: "Accounts") {
            if accounts.isEmpty {
                emptyState("No managed accounts for \(provider.capitalized).")
            }

            ForEach(accounts) { account in
                AccountTargetRow(
                    account: account,
                    active: account.active,
                    switching: store.switchingLocalId == account.id,
                    deleting: store.deletingLocalId == account.id,
                    resetting: store.resettingLocalId == account.id,
                    refreshing: store.refreshingProvider != nil,
                    confirmingDelete: store.confirmingDeleteTargetId == account.id,
                    confirmingReset: store.confirmingResetTargetId == account.id,
                    primary: quotaWindow(account, preferred: .short),
                    secondary: quotaWindow(account, preferred: .weekly),
                    accent: providerColor(account.provider),
                    switchAction: { Task { await store.switchAccount(account) } },
                    requestResetConfirmation: { store.confirmReset(account.id) },
                    cancelResetConfirmation: { store.cancelResetConfirmation() },
                    resetAction: { Task { await store.resetAccountUsageLimit(account) } },
                    requestDeleteConfirmation: { store.confirmDelete(account.id) },
                    cancelDeleteConfirmation: { store.cancelDeleteConfirmation() },
                    deleteAction: { Task { await store.deleteAccount(account) } }
                )
            }
        }
    }

    private func profileTargets(provider: String, profiles: [MenubarProfile], report: DashboardReport) -> some View {
        Card(title: "Profiles") {
            if profiles.isEmpty {
                emptyState("No profile targets reported for \(provider.capitalized).")
            }

            ForEach(profiles) { profile in
                ProfileTargetRow(
                    profile: profile,
                    active: profile.active,
                    switching: store.switchingLocalId == profile.id,
                    deleting: store.deletingLocalId == profile.id,
                    refreshing: store.refreshingProvider != nil,
                    confirmingDelete: store.confirmingDeleteTargetId == profile.id,
                    switchAction: { Task { await store.switchProfile(profile) } },
                    requestDeleteConfirmation: { store.confirmDelete(profile.id) },
                    cancelDeleteConfirmation: { store.cancelDeleteConfirmation() },
                    deleteAction: { Task { await store.deleteProfile(profile) } }
                )
            }
        }
    }

    private func diagnostics(provider: String, report: DashboardReport, accounts: [MenubarAccount]) -> some View {
        let providerDiagnostics = report.accounts.diagnostics + accounts.compactMap(\.diagnostic)
        return Card(title: "Diagnostics") {
            if providerDiagnostics.isEmpty {
                emptyState("No diagnostics for \(provider.capitalized).")
            }

            ForEach(Array(providerDiagnostics.enumerated()), id: \.offset) { _, diagnostic in
                DiagnosticView(diagnostic: diagnostic)
            }
        }
    }

    private func footer(providers: [String], selectedProvider: String?) -> some View {
        HStack(spacing: 10) {
            Text(selectedProvider.map { "\($0.capitalized) tools" } ?? "OpenMux tools")
                .font(.caption)
                .foregroundStyle(.secondary)

            Button("Manage in CLI") {
                copyCliHelpAndOpenTerminal(provider: selectedProvider ?? providers.first ?? "codex")
            }
            .buttonStyle(CompactCommandButtonStyle())

            Spacer()

            Button("Quit") { NSApplication.shared.terminate(nil) }
                .buttonStyle(CompactCommandButtonStyle())
        }
        .padding()
    }

    @ViewBuilder
    private func emptyState(_ text: String) -> some View {
        Text(text)
            .font(.caption)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func providerDot(_ provider: String) -> some View {
        Circle()
            .fill(providerColor(provider))
            .frame(width: 8, height: 8)
            .accessibilityHidden(true)
    }

    private func providerBadge(_ provider: String) -> some View {
        ZStack {
            Circle()
                .fill(providerColor(provider).opacity(0.22))
            ProviderIcon(provider: provider, size: 13)
                .foregroundStyle(providerColor(provider))
        }
        .frame(width: 30, height: 30)
        .accessibilityHidden(true)
    }

    private func headerSubtitle(_ report: DashboardReport, stale: Bool) -> String {
        let prefix = stale ? "Stale" : "Updated"
        return "\(prefix) \(timeLabel(report.generatedAtUnix))"
    }

    private func providerNames(_ report: DashboardReport) -> [String] {
        if !report.accounts.providers.isEmpty {
            return report.accounts.providers
        }
        return Array(Set(report.accounts.accounts.map(\.provider) + report.accounts.profiles.map(\.provider))).sorted()
    }

    private func accounts(for provider: String, in report: DashboardReport) -> [MenubarAccount] {
        report.accounts.accounts
            .filter { $0.provider == provider }
            .sorted { $0.displayNumber == $1.displayNumber ? $0.localId < $1.localId : $0.displayNumber < $1.displayNumber }
    }

    private func profiles(for provider: String, in report: DashboardReport) -> [MenubarProfile] {
        report.accounts.profiles
            .filter { $0.provider == provider }
            .sorted { $0.displayNumber == $1.displayNumber ? $0.name < $1.name : $0.displayNumber < $1.displayNumber }
    }

    private func providerUsage(_ provider: String, in report: DashboardReport) -> UsageSummary {
        report.providerUsage.first { $0.provider == provider }?.usage ?? report.usage
    }

    private func providerView(_ provider: String, in report: DashboardReport) -> ProviderView? {
        report.providerViews?.first { $0.provider == provider }
    }

    private func providerAttentionCount(_ report: DashboardReport) -> Int {
        (report.providerViews ?? []).filter(isProviderAttention).count
    }

    private func isProviderAttention(_ view: ProviderView) -> Bool {
        view.statusTone == "warning" || view.statusTone == "danger"
    }

    private var verticalRule: some View {
        Rectangle()
            .fill(Color.primary.opacity(0.12))
            .frame(width: 1, height: 42)
            .padding(.horizontal, 10)
    }

    private func lowestQuotaSummary(_ accounts: [MenubarAccount]) -> (raw: UInt32, percent: String, label: String)? {
        let candidates = accounts.compactMap { account -> (UInt32, String)? in
            guard let remaining = account.quota?.primaryWindow?.remainingPercentX100 else { return nil }
            let window = account.quota?.primaryWindow?.label ?? "quota"
            return (remaining, "\(account.provider.capitalized) \(window)")
        }
        guard let lowest = candidates.min(by: { $0.0 < $1.0 }) else { return nil }
        return (lowest.0, "\(Int(lowest.0) / 100)%", lowest.1)
    }

    private func timeAgo(_ timestamp: UInt64) -> String {
        let seconds = max(0, Int(Date().timeIntervalSince1970) - Int(timestamp))
        if seconds < 60 { return "\(seconds)s ago" }
        let minutes = seconds / 60
        if minutes < 60 { return "\(minutes)m ago" }
        let hours = minutes / 60
        if hours < 24 { return "\(hours)h ago" }
        return "\(hours / 24)d ago"
    }

    private func timeAgo(_ timestamp: Int64?) -> String {
        guard let timestamp else { return "not refreshed" }
        return timeAgo(UInt64(max(0, timestamp)))
    }

    private func activeAccount(_ accounts: [MenubarAccount]) -> MenubarAccount? {
        accounts.first(where: \.active)
    }

    private func activeProfile(_ profiles: [MenubarProfile]) -> MenubarProfile? {
        profiles.first(where: \.active)
    }

    private func activeTargetLabel(accounts: [MenubarAccount], profiles: [MenubarProfile]) -> String {
        if let account = activeAccount(accounts) {
            return hidePersonalIdentifiers ? "#\(account.displayNumber) Account" : account.shortLabel
        }
        if let profile = activeProfile(profiles) {
            return hidePersonalIdentifiers ? "#\(profile.displayNumber) Profile" : profile.displayLabel
        }
        return "-"
    }

    private func activeTargetLabel(account: MenubarAccount?, profile: MenubarProfile?) -> String? {
        if let account {
            return hidePersonalIdentifiers ? "#\(account.displayNumber) Account" : account.shortLabel
        }
        if let profile {
            return hidePersonalIdentifiers ? "#\(profile.displayNumber) Profile" : profile.displayLabel
        }
        return nil
    }

    private func providerUpdatedLabel(accounts: [MenubarAccount], fallback: UInt64) -> String {
        let refreshed = accounts.compactMap { $0.quota?.refreshedAtUnix }.max()
        if let refreshed {
            return timeAgo(refreshed)
        }
        return timeAgo(fallback)
    }

    private func providerSecondaryText(accounts: [MenubarAccount], profiles: [MenubarProfile]) -> String {
        if accounts.isEmpty, profiles.isEmpty {
            return "No accounts"
        }
        let accountText = accounts.isEmpty ? "No accounts" : "\(accounts.count) account\(accounts.count == 1 ? "" : "s")"
        let profileText = profiles.isEmpty ? "No profiles" : "\(profiles.count) profile\(profiles.count == 1 ? "" : "s")"
        return "\(accountText) · \(profileText)"
    }

    private func topModelLabel(_ usage: UsageSummary) -> String {
        usage.topModel ?? usage.modelBreakdown.first?.model ?? "No model"
    }

    private func coverageLabel(_ usage: UsageSummary) -> String {
        usage.coverage.status.capitalized
    }

    private func tokenText(_ tokens: UInt64) -> String {
        if tokens >= 1_000_000 {
            return String(format: "%.1fM", Double(tokens) / 1_000_000.0)
        }
        if tokens >= 1_000 {
            return String(format: "%.1fk", Double(tokens) / 1_000.0)
        }
        return "\(tokens)"
    }

    private func lowestQuota(_ accounts: [MenubarAccount]) -> UInt32? {
        accounts.compactMap { $0.quota?.primaryWindow?.remainingPercentX100 }.min()
    }

    private enum WindowPreference {
        case short
        case weekly
    }

    private func quotaWindow(_ account: MenubarAccount, preferred: WindowPreference) -> QuotaWindow? {
        let windows = account.quota?.windows ?? []
        switch preferred {
        case .short:
            return windows.first { window in
                let text = "\(window.id) \(window.label)".lowercased()
                return text.contains("5h") || text.contains("session") || text.contains("short")
            } ?? account.quota?.primaryWindow
        case .weekly:
            return windows.first { window in
                let text = "\(window.id) \(window.label)".lowercased()
                return text.contains("7d") || text.contains("week")
            } ?? windows.first { $0.id != account.quota?.primaryWindow?.id }
        }
    }

    private func timeLabel(_ timestamp: UInt64) -> String {
        timeLabel(Int64(timestamp))
    }

    private func timeLabel(_ timestamp: Int64) -> String {
        let date = Date(timeIntervalSince1970: TimeInterval(timestamp))
        return date.formatted(date: .omitted, time: .shortened)
    }

    private func copyCliHelpAndOpenTerminal(provider: String) {
        let help = """
        omx login \(provider)
        omx import \(provider) --file provider.toml
        omx alias \(provider) <selector> <alias>
        omx doctor \(provider)
        omx save \(provider) --alias recovery
        omx use \(provider) <selector>
        omx remove \(provider) <selector>
        """
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(help, forType: .string)
        NSWorkspace.shared.open(URL(fileURLWithPath: "/Applications/Utilities/Terminal.app"))
    }

    private var cardBackground: Color {
        colorScheme == .dark ? Color.white.opacity(0.08) : Color.white.opacity(0.82)
    }

    private var headerIconBackground: Color {
        colorScheme == .dark ? Color.white.opacity(0.10) : Color.black.opacity(0.055)
    }

    private var primaryText: Color {
        colorScheme == .dark ? Color.white.opacity(0.95) : Color.primary
    }

    private var secondaryText: Color {
        colorScheme == .dark ? Color.white.opacity(0.68) : Color.secondary
    }

    private func providerColor(_ provider: String) -> Color {
        switch provider.lowercased() {
        case "codex": return .green
        case "claude": return .orange
        case "gemini": return .blue
        default: return .purple
        }
    }

    private func toneColor(_ tone: String?) -> Color {
        switch tone {
        case .some("success"): return .green
        case .some("warning"): return .orange
        case .some("danger"): return .red
        default: return .secondary
        }
    }
}

struct IconFeedbackButtonStyle: ButtonStyle {
    var tint: Color = .primary

    func makeBody(configuration: Configuration) -> some View {
        PressableChrome(
            configuration: configuration,
            tint: tint,
            horizontalPadding: 0,
            verticalPadding: 0,
            minWidth: 28,
            minHeight: 28,
            cornerRadius: 7
        )
    }
}

private struct CompactCommandButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        PressableChrome(
            configuration: configuration,
            tint: .accentColor,
            horizontalPadding: 10,
            verticalPadding: 5,
            minWidth: 0,
            minHeight: 0,
            cornerRadius: 6
        )
        .font(.caption.weight(.semibold))
    }
}

private struct PanelRowButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        PressableChrome(
            configuration: configuration,
            tint: .primary,
            horizontalPadding: 8,
            verticalPadding: 7,
            minWidth: 0,
            minHeight: 0,
            cornerRadius: 8
        )
    }
}

private struct PressableChrome: View {
    @Environment(\.isEnabled) private var isEnabled

    let configuration: ButtonStyle.Configuration
    let tint: Color
    let horizontalPadding: CGFloat
    let verticalPadding: CGFloat
    let minWidth: CGFloat
    let minHeight: CGFloat
    let cornerRadius: CGFloat

    var body: some View {
        configuration.label
            .padding(.horizontal, horizontalPadding)
            .padding(.vertical, verticalPadding)
            .frame(minWidth: minWidth, minHeight: minHeight)
            .foregroundStyle(tint)
            .background(
                tint.opacity(backgroundOpacity),
                in: RoundedRectangle(cornerRadius: cornerRadius)
            )
            .scaleEffect(configuration.isPressed ? 0.96 : 1)
            .opacity(isEnabled ? 1 : 0.45)
            .contentShape(RoundedRectangle(cornerRadius: cornerRadius))
            .animation(.smooth(duration: 0.12), value: configuration.isPressed)
    }

    private var backgroundOpacity: Double {
        if configuration.isPressed { return 0.18 }
        return 0
    }
}

struct TargetRowFeedback: ViewModifier {
    let active: Bool
    let accent: Color

    func body(content: Content) -> some View {
        content
            .padding(.horizontal, 6)
            .background(
                accent.opacity(active ? 0.10 : 0),
                in: RoundedRectangle(cornerRadius: 8)
            )
            .animation(.smooth(duration: 0.14), value: active)
    }
}

private struct Card<Content: View>: View {
    @Environment(\.colorScheme) private var colorScheme
    private let title: String?
    private let content: Content

    init(title: String? = nil, @ViewBuilder content: () -> Content) {
        self.title = title
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if let title {
                Text(title)
                    .font(.headline)
            }
            content
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(background, in: RoundedRectangle(cornerRadius: 10))
        .overlay(
            RoundedRectangle(cornerRadius: 10)
                .stroke(Color.primary.opacity(colorScheme == .dark ? 0.12 : 0.08), lineWidth: 1)
        )
    }

    private var background: Color {
        colorScheme == .dark ? Color.white.opacity(0.08) : Color.white.opacity(0.86)
    }
}

private struct MetricCell: View {
    let label: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(value)
                .font(.title3.monospacedDigit().bold())
                .lineLimit(1)
            Text(label)
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(8)
        .background(Color.primary.opacity(0.045), in: RoundedRectangle(cornerRadius: 8))
    }
}

private struct OverviewStat: View {
    let value: String
    let label: String
    var color: Color = .primary

    var body: some View {
        VStack(spacing: 4) {
            Text(value)
                .font(.title2.monospacedDigit().bold())
                .foregroundStyle(color)
                .lineLimit(1)
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)
        }
        .frame(maxWidth: .infinity)
    }
}

private struct ProviderMiniColumn: View {
    let label: String
    let value: String
    let color: Color

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(label)
                .font(.caption2)
                .foregroundStyle(.secondary)
                .lineLimit(1)
            Text(value)
                .font(.caption.monospacedDigit().weight(.semibold))
                .foregroundStyle(color)
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct UsageMetric: View {
    let value: String
    let label: String
    let detail: String
    let color: Color

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(value)
                .font(.title3.monospacedDigit().bold())
                .foregroundStyle(color)
                .lineLimit(1)
                .minimumScaleFactor(0.72)
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(detail)
                .font(.caption2)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct StatusPill: View {
    let text: String
    let color: Color

    var body: some View {
        Text(text)
            .font(.caption2.monospacedDigit().weight(.semibold))
            .foregroundStyle(color)
            .padding(.horizontal, 7)
            .padding(.vertical, 3)
            .background(color.opacity(0.13), in: Capsule())
    }
}
