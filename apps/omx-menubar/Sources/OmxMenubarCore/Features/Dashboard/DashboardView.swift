import AppKit
import SwiftUI

struct DashboardView: View {
    @ObservedObject var store: AppStore
    @AppStorage("dev.openmux.menubar.trayDisplayMode") private var trayDisplayMode = "text"
    @AppStorage("dev.openmux.menubar.backgroundRefreshCadence") private var refreshCadence = 300
    @Environment(\.colorScheme) private var colorScheme

    private let width: CGFloat = 392
    private let height: CGFloat = 640

    var body: some View {
        Group {
            switch store.state {
            case .loading:
                loadingView
            case .failed(let lastGood, let message):
                if let lastGood {
                    content(lastGood, stale: true, errorMessage: message)
                } else {
                    failedView(message)
                }
            case .ready(let report, let stale):
                content(report, stale: stale, errorMessage: store.statusMessage)
            }
        }
        .frame(width: width, height: height)
        .background(shellBackground)
        .animation(.smooth(duration: 0.18), value: store.selectedProvider)
        .animation(.smooth(duration: 0.16), value: store.refreshingProvider)
        .animation(.smooth(duration: 0.16), value: store.switchingLocalId)
        .animation(.smooth(duration: 0.16), value: store.deletingLocalId)
        .animation(.smooth(duration: 0.16), value: store.confirmingDeleteTargetId)
    }

    private var loadingView: some View {
        VStack(alignment: .leading, spacing: 14) {
            headerSkeleton
            Card {
                ProgressView()
                Text("Loading dashboard")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
        }
        .padding()
    }

    private func failedView(_ message: String) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("OpenMux")
                .font(.title3.bold())
            Card {
                Text("Backend unavailable")
                    .font(.headline)
                Text(message)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(4)
                Button("Retry") { Task { await store.load() } }
                    .buttonStyle(CompactCommandButtonStyle())
            }
            Spacer()
            footer(providers: [], selectedProvider: nil)
        }
        .padding()
    }

    private func content(_ report: DashboardReport, stale: Bool, errorMessage: String?) -> some View {
        let providers = providerNames(report)
        let currentProvider = store.selectedProvider.flatMap { providers.contains($0) ? $0 : nil }

        return VStack(alignment: .leading, spacing: 0) {
            header(report, stale: stale)
            providerSelector(providers: providers, selected: currentProvider)
                .padding(.horizontal)
                .padding(.bottom, 10)

            Divider()

            ScrollView {
                VStack(alignment: .leading, spacing: 12) {
                    if let errorMessage {
                        banner(errorMessage, color: statusColor(store.statusKind))
                    }

                    if let currentProvider {
                        providerPage(provider: currentProvider, report: report)
                    } else {
                        overview(report)
                    }
                }
                .padding()
                .id(currentProvider ?? "overview")
                .transition(.opacity.combined(with: .move(edge: .trailing)))
            }

            Divider()
            footer(providers: providers, selectedProvider: currentProvider)
        }
    }

    private var headerSkeleton: some View {
        HStack(alignment: .center, spacing: 12) {
            headerIcon(provider: "openmux", stale: false)

            VStack(alignment: .leading, spacing: 4) {
                Text("OpenMux")
                    .font(.title3.bold())
                Text("Loading...")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer()
        }
        .padding()
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

                Menu {
                    Picker("Tray display", selection: $trayDisplayMode) {
                        Text("Text").tag("text")
                        Text("Icon only").tag("icon_only")
                    }
                    Picker("Background refresh", selection: $refreshCadence) {
                        Text("5 min").tag(300)
                        Text("15 min").tag(900)
                        Text("30 min").tag(1800)
                    }
                } label: {
                    Image(systemName: "gearshape")
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .buttonStyle(IconFeedbackButtonStyle())
                .help("Settings")
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

    private func providerSelector(providers: [String], selected: String?) -> some View {
        HStack(spacing: 4) {
            selectorButton("Overview", active: selected == nil) {
                store.selectedProvider = nil
            }

            ForEach(providers, id: \.self) { provider in
                selectorButton(provider.capitalized, active: selected == provider) {
                    store.selectedProvider = provider
                }
            }
        }
        .padding(4)
        .background(selectorBackground, in: Capsule())
    }

    private func selectorButton(_ title: String, active: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 5) {
                if title != "Overview" {
                    Circle()
                        .fill(providerColor(title))
                        .frame(width: 6, height: 6)
                }
                Text(title)
                    .lineLimit(1)
            }
            .font(.caption.weight(active ? .semibold : .regular))
            .foregroundStyle(active ? primaryText : secondaryText)
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
            .frame(maxWidth: .infinity)
            .background(active ? selectorActive : Color.clear, in: Capsule())
        }
        .buttonStyle(SegmentedFeedbackButtonStyle(active: active))
    }

    private func overview(_ report: DashboardReport) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            poolSummary(report)
            providerSummaries(report)
            usageSummary(report.usage, title: "Token Usage", subtitle: "Local parsed usage across providers")
        }
    }

    private func poolSummary(_ report: DashboardReport) -> some View {
        let accounts = report.accounts.accounts
        let profiles = report.accounts.profiles
        let providers = providerNames(report)
        let activeProviders = Set(
            accounts.filter(\.active).map(\.provider)
                + profiles.filter(\.active).map(\.provider)
        ).count
        let stale = accounts.filter { $0.status == "stale" || $0.diagnostic != nil }.count
            + profiles.filter { $0.status == "stale" || $0.diagnostic != nil }.count
        let lowest = lowestQuotaSummary(accounts)

        return Card {
            HStack(alignment: .top, spacing: 10) {
                Image(systemName: "person.2.fill")
                    .font(.title3.weight(.semibold))
                    .foregroundStyle(.purple)
                    .frame(width: 26)
                VStack(alignment: .leading, spacing: 2) {
                    Text("All Accounts")
                        .font(.headline)
                    Text("Whole-pool aggregation")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            HStack(spacing: 0) {
                OverviewStat(value: "\(providers.count)", label: "Providers")
                verticalRule
                OverviewStat(value: "\(accounts.count)", label: "Accounts")
                verticalRule
                OverviewStat(value: "\(profiles.count)", label: "Profiles")
                verticalRule
                OverviewStat(value: "\(stale)", label: "Stale", color: stale > 0 ? .orange : .green)
                verticalRule
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 5) {
                        Image(systemName: "shield.lefthalf.filled")
                            .font(.caption)
                        Text("Lowest quota")
                            .font(.caption)
                    }
                    .foregroundStyle(.secondary)

                    Text(lowest?.percent ?? "Unknown")
                        .font(.title3.monospacedDigit().bold())
                        .foregroundStyle(lowest == nil ? secondaryText : quotaColor(lowest?.raw))
                    Text(lowest?.label ?? "No quota data")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(.top, 4)

            HStack(spacing: 6) {
                Image(systemName: "clock")
                    .font(.caption)
                Text("Last refreshed \(timeAgo(report.generatedAtUnix))")
                if stale > 0 {
                    Text("·")
                    Text("Some data may be stale")
                        .foregroundStyle(.orange)
                } else if activeProviders > 0 {
                    Text("·")
                    Text("\(activeProviders) active provider\(activeProviders == 1 ? "" : "s")")
                }
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
    }

    private func providerSummaries(_ report: DashboardReport) -> some View {
        Card {
            HStack(alignment: .top, spacing: 10) {
                Image(systemName: "square.stack.3d.up.fill")
                    .font(.title3.weight(.semibold))
                    .foregroundStyle(.purple)
                    .frame(width: 26)
                VStack(alignment: .leading, spacing: 2) {
                    Text("Providers")
                        .font(.headline)
                    Text("Current selection and health")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            let providers = providerNames(report)
            if providers.isEmpty {
                emptyState("No providers reported by backend.")
            }

            ForEach(providers, id: \.self) { provider in
                let accounts = accounts(for: provider, in: report)
                let profiles = profiles(for: provider, in: report)
                Button {
                    store.selectedProvider = provider
                } label: {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack(spacing: 10) {
                            providerBadge(provider)
                            VStack(alignment: .leading, spacing: 3) {
                                Text(provider.capitalized)
                                    .font(.subheadline.weight(.semibold))
                                    .foregroundStyle(primaryText)
                                Text(providerSecondaryText(accounts: accounts, profiles: profiles))
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .lineLimit(1)
                            }
                            Spacer()
                            StatusPill(
                                text: providerStatusText(accounts: accounts, profiles: profiles),
                                color: providerStatusColor(accounts: accounts, profiles: profiles)
                            )
                            Image(systemName: "chevron.right")
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                        }

                        HStack(spacing: 0) {
                            ProviderMiniColumn(label: "Current", value: activeTargetLabel(accounts: accounts, profiles: profiles, report: report), color: activeTargetLabel(accounts: accounts, profiles: profiles, report: report) == "-" ? secondaryText : .green)
                            verticalRule
                            ProviderMiniColumn(label: "Accounts", value: "\(accounts.count)", color: primaryText)
                            verticalRule
                            ProviderMiniColumn(label: "Profiles", value: "\(profiles.count)", color: primaryText)
                            verticalRule
                            ProviderMiniColumn(
                                label: accounts.isEmpty ? "State" : "Updated",
                                value: accounts.isEmpty ? "Planned" : providerUpdatedLabel(accounts: accounts, fallback: report.generatedAtUnix),
                                color: accounts.isEmpty ? secondaryText : primaryText
                            )
                        }
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(PanelRowButtonStyle())
                .padding(.vertical, 7)

                if provider != providers.last {
                    Divider()
                        .opacity(0.45)
                }
            }
        }
    }

    private func providerPage(provider: String, report: DashboardReport) -> some View {
        let accounts = accounts(for: provider, in: report)
        let profiles = profiles(for: provider, in: report)
        let active = activeAccount(accounts, report: report)
        let activeProfile = activeProfile(profiles, report: report)

        let usage = providerUsage(provider, in: report)

        return VStack(alignment: .leading, spacing: 12) {
            providerOverview(provider: provider, accounts: accounts, profiles: profiles, active: active, activeProfile: activeProfile, usage: usage)
            accountTargets(provider: provider, accounts: accounts, report: report)
            profileTargets(provider: provider, profiles: profiles, report: report)
            usageSummary(usage, title: "\(provider.capitalized) Token Usage", subtitle: "Provider local usage, not account quota")
            diagnostics(provider: provider, report: report, accounts: accounts)
        }
    }

    private func providerOverview(provider: String, accounts: [MenubarAccount], profiles: [MenubarProfile], active: MenubarAccount?, activeProfile: MenubarProfile?, usage: UsageSummary) -> some View {
        let targetLabel = active?.shortLabel ?? activeProfile?.displayLabel
        let lowest = lowestQuota(accounts).map { "\(Int($0) / 100)%" } ?? "unknown"
        let alerts = accounts.filter { $0.status != "healthy" || $0.diagnostic != nil }.count
            + profiles.filter { $0.status != "healthy" || $0.diagnostic != nil }.count

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
                let isActive = isActiveTarget(accountKey: account.accountKey, kind: account.targetKind, report: report)
                AccountTargetRow(
                    account: account,
                    active: isActive,
                    switching: store.switchingLocalId == account.id,
                    deleting: store.deletingLocalId == account.id,
                    refreshing: store.refreshingProvider != nil,
                    confirmingDelete: store.confirmingDeleteTargetId == account.id,
                    primary: quotaWindow(account, preferred: .short),
                    secondary: quotaWindow(account, preferred: .weekly),
                    accent: providerColor(account.provider),
                    switchAction: { Task { await store.switchAccount(account) } },
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
                let isActive = isActiveTarget(accountKey: profile.accountKey, kind: profile.targetKind, report: report)
                ProfileTargetRow(
                    profile: profile,
                    active: isActive,
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

    private func usageSummary(_ usage: UsageSummary, title: String, subtitle: String) -> some View {
        Card {
            HStack(alignment: .top, spacing: 10) {
                Image(systemName: "chart.bar.fill")
                    .font(.title3.weight(.semibold))
                    .foregroundStyle(.secondary)
                    .frame(width: 26)
                VStack(alignment: .leading, spacing: 4) {
                    Text(title)
                        .font(.headline)
                    Text(subtitle)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Text("Today")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.purple)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 5)
                    .background(Color.purple.opacity(0.12), in: Capsule())
            }

            HStack(spacing: 0) {
                UsageMetric(value: tokenText(usage.totalTokens), label: "tokens", detail: "Today", color: primaryText)
                verticalRule
                UsageMetric(value: topModelLabel(usage), label: "Top model", detail: usage.topClient ?? "Local", color: primaryText)
                verticalRule
                UsageMetric(value: coverageLabel(usage), label: "Coverage", detail: "Local parsed", color: usageColor(usage))
            }

            ModelUsageBars(models: usage.modelBreakdown)
                .frame(height: 116)
        }
    }

    private func diagnostics(provider: String, report: DashboardReport, accounts: [MenubarAccount]) -> some View {
        let providerDiagnostics = report.accounts.diagnostics + accounts.compactMap(\.diagnostic)
        return Card(title: "Diagnostics") {
            if providerDiagnostics.isEmpty {
                emptyState("No diagnostics for \(provider.capitalized).")
            }

            ForEach(Array(providerDiagnostics.enumerated()), id: \.offset) { _, diagnostic in
                VStack(alignment: .leading, spacing: 2) {
                    Text(diagnostic.code)
                        .font(.caption.weight(.semibold))
                    Text(diagnostic.message)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(3)
                }
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

    private func banner(_ message: String, color: Color) -> some View {
        Text(message)
            .font(.caption)
            .foregroundStyle(color)
            .lineLimit(3)
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(color.opacity(0.12), in: RoundedRectangle(cornerRadius: 8))
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
            Image(systemName: providerIcon(provider))
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(providerColor(provider))
        }
        .frame(width: 30, height: 30)
        .accessibilityHidden(true)
    }

    private func providerIcon(_ provider: String) -> String {
        switch provider.lowercased() {
        case "codex": return "brain.head.profile"
        case "claude": return "sparkle"
        case "gemini": return "diamond.fill"
        default: return "circle.grid.2x2.fill"
        }
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

    private func quotaColor(_ raw: UInt32?) -> Color {
        guard let raw else { return secondaryText }
        if raw <= 1_500 { return .red }
        if raw <= 4_000 { return .orange }
        return .green
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

    private func isActiveTarget(accountKey: String, kind: String, report: DashboardReport) -> Bool {
        if let activeKey = report.accounts.activeTargetKey {
            return accountKey == activeKey
        }
        return false
    }

    private func activeAccount(_ accounts: [MenubarAccount], report: DashboardReport) -> MenubarAccount? {
        accounts.first { isActiveTarget(accountKey: $0.accountKey, kind: $0.targetKind, report: report) }
    }

    private func activeProfile(_ profiles: [MenubarProfile], report: DashboardReport) -> MenubarProfile? {
        profiles.first { isActiveTarget(accountKey: $0.accountKey, kind: $0.targetKind, report: report) }
    }

    private func activeTargetLabel(accounts: [MenubarAccount], profiles: [MenubarProfile], report: DashboardReport) -> String {
        if let account = activeAccount(accounts, report: report) {
            return account.shortLabel
        }
        if let profile = activeProfile(profiles, report: report) {
            return profile.displayLabel
        }
        return "-"
    }

    private func providerUpdatedLabel(accounts: [MenubarAccount], fallback: UInt64) -> String {
        let refreshed = accounts.compactMap { $0.quota?.refreshedAtUnix }.max()
        if let refreshed {
            return timeAgo(refreshed)
        }
        return timeAgo(fallback)
    }

    private func providerStatusText(accounts: [MenubarAccount], profiles: [MenubarProfile]) -> String {
        guard !accounts.isEmpty || !profiles.isEmpty else { return "Planned" }
        if accounts.contains(where: { $0.status == "exhausted" || $0.status == "unavailable" })
            || profiles.contains(where: { $0.status == "exhausted" || $0.status == "unavailable" })
        {
            return "Alert"
        }
        if accounts.contains(where: { $0.status == "limited" || $0.status == "stale" || $0.diagnostic != nil })
            || profiles.contains(where: { $0.status == "limited" || $0.status == "stale" || $0.diagnostic != nil })
        {
            return "Stale"
        }
        return "OK"
    }

    private func providerStatusColor(accounts: [MenubarAccount], profiles: [MenubarProfile]) -> Color {
        switch providerStatusText(accounts: accounts, profiles: profiles) {
        case "OK": return .green
        case "Planned": return .secondary
        case "Alert": return .red
        default: return .orange
        }
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

    private func usageColor(_ usage: UsageSummary) -> Color {
        usage.coverage.status == "complete" ? .green : .orange
    }

    private func providerSummaryText(accounts: [MenubarAccount], profiles: [MenubarProfile]) -> String {
        let active = accounts.first(where: \.active)?.shortLabel
            ?? profiles.first(where: \.active)?.displayLabel
            ?? "no active target"
        return "\(active) · \(accounts.count) accounts · \(profiles.count) profiles"
    }

    private func providerHealth(accounts: [MenubarAccount], profiles: [MenubarProfile]) -> String {
        guard !accounts.isEmpty || !profiles.isEmpty else { return "empty" }
        if let lowest = lowestQuota(accounts) {
            return "\(Int(lowest) / 100)%"
        }
        if accounts.contains(where: { $0.diagnostic != nil || $0.status != "healthy" })
            || profiles.contains(where: { $0.diagnostic != nil || $0.status != "healthy" })
        {
            return "alert"
        }
        return "ok"
    }

    private func providerHealthColor(accounts: [MenubarAccount], profiles: [MenubarProfile]) -> Color {
        if accounts.contains(where: { $0.status == "exhausted" || $0.status == "unavailable" })
            || profiles.contains(where: { $0.status == "exhausted" || $0.status == "unavailable" })
        {
            return .red
        }
        if accounts.contains(where: { $0.status == "limited" || $0.status == "stale" || $0.diagnostic != nil })
            || profiles.contains(where: { $0.status == "limited" || $0.status == "stale" || $0.diagnostic != nil })
        {
            return .orange
        }
        return .green
    }

    private func statusColor(_ status: String?) -> Color {
        switch status {
        case "success": return .green
        case "skipped": return .orange
        case "failed": return .red
        default: return .orange
        }
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
        omx remove \(provider) <selector>
        """
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(help, forType: .string)
        NSWorkspace.shared.open(URL(fileURLWithPath: "/Applications/Utilities/Terminal.app"))
    }

    private var shellBackground: Color {
        colorScheme == .dark ? Color(red: 0.18, green: 0.16, blue: 0.34) : Color(nsColor: .windowBackgroundColor)
    }

    private var cardBackground: Color {
        colorScheme == .dark ? Color.white.opacity(0.08) : Color.white.opacity(0.82)
    }

    private var selectorBackground: Color {
        colorScheme == .dark ? Color.white.opacity(0.07) : Color.black.opacity(0.05)
    }

    private var selectorActive: Color {
        colorScheme == .dark ? Color.white.opacity(0.14) : Color.white
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
}

private struct IconFeedbackButtonStyle: ButtonStyle {
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

private struct SegmentedFeedbackButtonStyle: ButtonStyle {
    let active: Bool

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(
                Color.primary.opacity(!active && configuration.isPressed ? 0.08 : 0),
                in: Capsule()
            )
            .scaleEffect(configuration.isPressed ? 0.985 : 1)
            .animation(.smooth(duration: 0.12), value: configuration.isPressed)
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

private struct TargetRowFeedback: ViewModifier {
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

private struct ModelUsageBars: View {
    let models: [UsageModelBreakdown]

    var body: some View {
        if models.isEmpty {
            Text("No model usage today")
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
                .background(Color.primary.opacity(0.035), in: RoundedRectangle(cornerRadius: 8))
        } else {
            VStack(alignment: .leading, spacing: 7) {
                ZStack(alignment: .bottomLeading) {
                    VStack(spacing: 0) {
                        ForEach(0..<4, id: \.self) { _ in
                            Rectangle()
                                .fill(Color.primary.opacity(0.07))
                                .frame(height: 1)
                            Spacer()
                        }
                    }

                    HStack(alignment: .bottom, spacing: 8) {
                        ForEach(Array(models.enumerated()), id: \.offset) { index, model in
                            VStack(spacing: 4) {
                                RoundedRectangle(cornerRadius: 3)
                                    .fill(modelColor(model.model, index: index))
                                    .frame(height: barHeight(model.totalTokens))
                                    .overlay(alignment: .top) {
                                        RoundedRectangle(cornerRadius: 3)
                                            .fill(Color.white.opacity(0.22))
                                            .frame(height: 4)
                                    }
                                Text(shortModel(model.model))
                                    .font(.caption2)
                                    .foregroundStyle(.secondary)
                                    .lineLimit(1)
                            }
                            .frame(maxWidth: .infinity)
                            .accessibilityLabel("\(model.model) \(model.totalTokens) tokens")
                        }
                    }
                }
                .padding(.horizontal, 8)
                .padding(.top, 8)
                .background(Color.primary.opacity(0.035), in: RoundedRectangle(cornerRadius: 8))

                HStack(spacing: 10) {
                    ForEach(Array(models.prefix(4).enumerated()), id: \.offset) { index, model in
                        HStack(spacing: 4) {
                            Circle()
                                .fill(modelColor(model.model, index: index))
                                .frame(width: 7, height: 7)
                            Text(shortModel(model.model))
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                                .lineLimit(1)
                        }
                    }
                    Spacer()
                }
            }
        }
    }

    private var maxTokens: UInt64 {
        models.map(\.totalTokens).max() ?? 1
    }

    private func barHeight(_ tokens: UInt64) -> CGFloat {
        10 + CGFloat(tokens) / CGFloat(maxTokens) * 42
    }

    private func shortModel(_ model: String) -> String {
        model.replacingOccurrences(of: "claude-", with: "")
            .replacingOccurrences(of: "gpt-", with: "g")
            .replacingOccurrences(of: "gemini-", with: "gm-")
    }

    private func modelColor(_ model: String, index: Int) -> Color {
        let lower = model.lowercased()
        if lower.contains("claude") { return .orange }
        if lower.contains("gemini") { return .blue }
        if lower.contains("gpt") || lower.contains("codex") { return .green }
        return [.purple, .pink, .teal, .indigo][index % 4]
    }
}

private struct AccountTargetRow: View {
    let account: MenubarAccount
    let active: Bool
    let switching: Bool
    let deleting: Bool
    let refreshing: Bool
    let confirmingDelete: Bool
    let primary: QuotaWindow?
    let secondary: QuotaWindow?
    let accent: Color
    let switchAction: () -> Void
    let requestDeleteConfirmation: () -> Void
    let cancelDeleteConfirmation: () -> Void
    let deleteAction: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack(alignment: .center, spacing: 12) {
                HStack(spacing: 6) {
                    Text("#\(account.displayNumber)")
                        .font(.caption.monospacedDigit())
                        .foregroundStyle(.secondary)
                    Text(account.shortLabel)
                        .font(.subheadline.weight(.semibold))
                        .lineLimit(1)
                        .truncationMode(.tail)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .help(account.accountLabel ?? account.shortLabel)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .clipped()

                ActionRail(
                    active: active,
                    switching: switching,
                    deleting: deleting,
                    refreshing: refreshing,
                    activeLabel: "Current account",
                    switchLabel: "Use this account",
                    deleteLabel: "Delete account target",
                    switchAction: switchAction,
                    requestDeleteConfirmation: requestDeleteConfirmation,
                    deleteConfirmationBinding: deleteConfirmationBinding,
                    deletePopover: {
                        DeleteConfirmPopover(
                            title: "Delete \(account.shortLabel)?",
                            message: "Removes the OpenMux target and its managed snapshot.",
                            deleteAction: deleteAction,
                            cancelAction: cancelDeleteConfirmation
                        )
                    }
                )
            }

            AccountMetaLine(plan: account.plan ?? account.status, refreshedAtUnix: account.quota?.refreshedAtUnix)
            if let diagnostic = account.diagnostic {
                Text("\(diagnostic.code): \(diagnostic.message)")
                    .font(.caption2)
                    .foregroundStyle(.orange)
                    .lineLimit(1)
                    .truncationMode(.tail)
            }
            UsageRailView(quota: account.quota, primary: primary, secondary: secondary, accent: accent)
        }
        .padding(.vertical, 9)
        .modifier(TargetRowFeedback(active: active, accent: accent))
        .accessibilityElement(children: .combine)
    }

    private var deleteConfirmationBinding: Binding<Bool> {
        Binding(
            get: { confirmingDelete },
            set: { isPresented in
                if isPresented {
                    requestDeleteConfirmation()
                } else {
                    cancelDeleteConfirmation()
                }
            }
        )
    }
}

private struct AccountMetaLine: View {
    let plan: String
    let refreshedAtUnix: Int64?

    var body: some View {
        HStack(spacing: 6) {
            Text(plan)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(.tail)
            Spacer(minLength: 8)
            Text(refreshTime)
                .font(.caption.monospacedDigit())
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .frame(width: 76, alignment: .trailing)
        }
    }

    private var refreshTime: String {
        guard let refreshedAtUnix else { return "-" }
        return shortDateTimeLabel(refreshedAtUnix)
    }
}

private struct ProfileTargetRow: View {
    let profile: MenubarProfile
    let active: Bool
    let switching: Bool
    let deleting: Bool
    let refreshing: Bool
    let confirmingDelete: Bool
    let switchAction: () -> Void
    let requestDeleteConfirmation: () -> Void
    let cancelDeleteConfirmation: () -> Void
    let deleteAction: () -> Void

    var body: some View {
        HStack(alignment: .center, spacing: 12) {
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    Text(profile.displayNumber > 0 ? "#\(profile.displayNumber)" : "Profile")
                        .font(.caption.monospacedDigit())
                        .foregroundStyle(.secondary)
                    Text(profile.displayLabel)
                        .font(.subheadline.weight(.semibold))
                        .lineLimit(1)
                        .truncationMode(.tail)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .help(profile.displayLabel)
                }
                Text(profile.secondaryLabel)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .clipped()

            ActionRail(
                active: active,
                switching: switching,
                deleting: deleting,
                refreshing: refreshing,
                activeLabel: "Current profile",
                switchLabel: "Use this profile",
                deleteLabel: "Delete profile target",
                switchAction: switchAction,
                requestDeleteConfirmation: requestDeleteConfirmation,
                deleteConfirmationBinding: deleteConfirmationBinding,
                deletePopover: {
                    DeleteConfirmPopover(
                        title: "Delete \(profile.displayLabel)?",
                        message: "Removes the OpenMux profile target and its managed secret if owned by OpenMux.",
                        deleteAction: deleteAction,
                        cancelAction: cancelDeleteConfirmation
                    )
                }
            )
        }
        .padding(.vertical, 7)
        .modifier(TargetRowFeedback(active: active, accent: .accentColor))
        .accessibilityElement(children: .combine)
    }

    private var deleteConfirmationBinding: Binding<Bool> {
        Binding(
            get: { confirmingDelete },
            set: { isPresented in
                if isPresented {
                    requestDeleteConfirmation()
                } else {
                    cancelDeleteConfirmation()
                }
            }
        )
    }
}

private struct ActionRail<DeletePopover: View>: View {
    let active: Bool
    let switching: Bool
    let deleting: Bool
    let refreshing: Bool
    let activeLabel: String
    let switchLabel: String
    let deleteLabel: String
    let switchAction: () -> Void
    let requestDeleteConfirmation: () -> Void
    let deleteConfirmationBinding: Binding<Bool>
    let deletePopover: () -> DeletePopover

    var body: some View {
        HStack(spacing: 4) {
            Button {
                switchAction()
            } label: {
                Image(systemName: active ? "checkmark.circle.fill" : switching ? "hourglass" : "arrow.right.circle")
                    .foregroundStyle(active ? .green : .secondary)
                    .frame(width: 28, height: 28)
            }
            .disabled(active || switching || refreshing || deleting)
            .buttonStyle(IconFeedbackButtonStyle(tint: active ? .green : .secondary))
            .help(active ? activeLabel : switching ? "Switching" : switchLabel)
            .accessibilityLabel(active ? activeLabel : switchLabel)

            Button {
                requestDeleteConfirmation()
            } label: {
                Image(systemName: deleting ? "hourglass" : "trash")
                    .frame(width: 28, height: 28)
            }
            .disabled(deleting || refreshing)
            .buttonStyle(IconFeedbackButtonStyle(tint: .red))
            .help(deleteLabel)
            .accessibilityLabel(deleteLabel)
            .popover(isPresented: deleteConfirmationBinding, arrowEdge: .trailing, content: deletePopover)
        }
        .frame(width: 60, alignment: .trailing)
    }
}

private struct DeleteConfirmPopover: View {
    let title: String
    let message: String
    let deleteAction: () -> Void
    let cancelAction: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title)
                .font(.headline)
            Text(message)
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)

            HStack {
                Spacer()
                Button("Cancel", action: cancelAction)
                    .keyboardShortcut(.cancelAction)
                Button("Delete", role: .destructive, action: deleteAction)
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(12)
        .frame(width: 260)
    }
}

private func clockLabel(_ timestamp: Int64) -> String {
    Date(timeIntervalSince1970: TimeInterval(timestamp))
        .formatted(date: .omitted, time: .shortened)
}

private func shortDateTimeLabel(_ timestamp: Int64) -> String {
    let date = Date(timeIntervalSince1970: TimeInterval(timestamp))
    let formatter = DateFormatter()
    formatter.dateFormat = "MM-dd HH:mm"
    return formatter.string(from: date)
}

private func resetLabel(_ window: QuotaWindow?) -> String {
    guard let timestamp = window?.resetAtUnix else { return "-" }
    return shortDateTimeLabel(timestamp)
}

private struct UsageRailView: View {
    let quota: Quota?
    let primary: QuotaWindow?
    let secondary: QuotaWindow?
    let accent: Color

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            QuotaProgressLine(window: primary, fallbackLabel: "5h", color: accent)
            QuotaProgressLine(window: secondary, fallbackLabel: "7d", color: .blue)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .help(helpText)
        .accessibilityLabel(helpText)
    }

    private var refreshedLabel: String {
        guard let timestamp = quota?.refreshedAtUnix else { return "refreshed -" }
        return "refreshed \(shortDateTimeLabel(timestamp))"
    }

    private var helpText: String {
        "\(lineHelp(primary, fallbackLabel: "5h")); \(lineHelp(secondary, fallbackLabel: "weekly")); \(refreshedLabel)"
    }

    private func lineHelp(_ window: QuotaWindow?, fallbackLabel: String) -> String {
        "\(window?.label ?? fallbackLabel) \(percentText(window)), \(resetLabel(window))"
    }

    private func percentText(_ window: QuotaWindow?) -> String {
        guard let remaining = window?.remainingPercentX100 else { return "--" }
        return "\(Int(remaining) / 100)%"
    }
}

private struct QuotaProgressLine: View {
    let window: QuotaWindow?
    let fallbackLabel: String
    let color: Color

    var body: some View {
        HStack(spacing: 6) {
            Text(label)
                .font(.caption2)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .frame(width: 28, alignment: .leading)
            GeometryReader { proxy in
                ZStack(alignment: .leading) {
                    Capsule()
                        .fill(Color.primary.opacity(0.12))
                    Capsule()
                        .fill(color)
                        .frame(width: max(3, proxy.size.width * fraction))
                }
            }
            .frame(height: 6)
            Text(percentText)
                .font(.caption2.monospacedDigit().weight(.semibold))
                .frame(width: 34, alignment: .trailing)
            Text(resetLabel(window))
                .font(.caption2.monospacedDigit())
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .frame(width: 76, alignment: .trailing)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .clipped()
    }

    private var label: String {
        let raw = window?.label ?? fallbackLabel
        return raw.lowercased().contains("week") ? "7d" : raw
    }

    private var fraction: Double {
        guard let remaining = window?.remainingPercentX100 else { return 0 }
        return max(0, min(1, Double(remaining) / 10_000.0))
    }

    private var percentText: String {
        guard let remaining = window?.remainingPercentX100 else { return "--" }
        return "\(Int(remaining) / 100)%"
    }
}
