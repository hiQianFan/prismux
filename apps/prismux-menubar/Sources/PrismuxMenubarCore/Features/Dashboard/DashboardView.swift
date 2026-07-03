import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct DashboardView: View {
    @ObservedObject var store: AppStore
    let report: DashboardReport
    let stale: Bool
    let onOpenSettings: (MenubarSettingsTab) -> Void

    @AppStorage("dev.prismux.menubar.hidePersonalIdentifiers") private var hidePersonalIdentifiers = false
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var importProvider: String?
    @State private var importName = ""
    @State private var importContent = ""
    @State private var importError: String?
    @State private var cliReady = false

    var body: some View {
        content(report, stale: stale)
            // Inline overlay inside the popover, NOT a .sheet: a sheet opens its
            // own modal window, and dismissing the transient popover while it is
            // up orphans the modal session and freezes all input on reopen.
            .overlay {
                if importProvider != nil {
                    importProfileOverlay
                }
            }
            .task { cliReady = CliToolStatus.detect().resolution == .ready }
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
            footer(report: report, stale: stale)
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
        let isRefreshing = store.refreshingProvider != nil || store.refreshingTargetId != nil
        return HStack(alignment: .center, spacing: 12) {
            headerIcon(provider: report.active?.provider ?? providerNames(report).first ?? "prismux", stale: stale)

            VStack(alignment: .leading, spacing: 4) {
                Text("Prismux")
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
                    if isRefreshing {
                        ProgressView()
                            .controlSize(.small)
                            .frame(width: 16, height: 16)
                    } else {
                        Image(systemName: "arrow.clockwise")
                    }
                }
                .buttonStyle(IconFeedbackButtonStyle())
                .disabled(isRefreshing)
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
            Image(systemName: stale ? "exclamationmark.triangle" : "triangle")
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
        let alerts = aggregatedDiagnostics(report)
        return VStack(alignment: .leading, spacing: 12) {
            Card(title: "Providers") {
                if providers.isEmpty {
                    emptyState("No providers reported by backend.")
                }
                ForEach(providers, id: \.self) { provider in
                    if let aggregate = providerAggregate(provider, in: report) {
                        ProviderSummaryCard(
                            aggregate: aggregate,
                            activeLabel: overviewActiveLabel(provider: provider, aggregate: aggregate, report: report),
                            onTap: { store.selectedProvider = provider }
                        )
                        if provider != providers.last {
                            Divider().opacity(0.4)
                        }
                    }
                }
            }

            if !alerts.isEmpty {
                Card(title: "Needs attention") {
                    ForEach(Array(alerts.enumerated()), id: \.offset) { _, diagnostic in
                        DiagnosticView(diagnostic: diagnostic)
                    }
                }
            }
        }
    }

    /// Masked active label for a provider's Overview row. Prefers the aggregate's
    /// active target, falling back to the local account/profile, and applies the
    /// same privacy masking the rest of the UI uses.
    private func overviewActiveLabel(provider: String, aggregate: ProviderAggregateView, report: DashboardReport) -> String? {
        if hidePersonalIdentifiers {
            let accounts = accounts(for: provider, in: report)
            let profiles = profiles(for: provider, in: report)
            return activeTargetLabel(account: activeAccount(accounts), profile: activeProfile(profiles))
        }
        return aggregate.activeTarget?.displayLabel
    }

    /// Provider diagnostics + dashboard-level diagnostics, concatenated. Provider
    /// diagnostics are already scoped by provider_id; dashboard diagnostics carry
    /// none, so the two sets don't overlap.
    private func aggregatedDiagnostics(_ report: DashboardReport) -> [Diagnostic] {
        let providerDiagnostics = report.aggregate.providerAggregates.flatMap(\.diagnostics)
        return providerDiagnostics + report.aggregate.diagnostics
    }

    private func providerPage(provider: String, report: DashboardReport) -> some View {
        let accounts = accounts(for: provider, in: report)
        let profiles = profiles(for: provider, in: report)

        return VStack(alignment: .leading, spacing: 12) {
            accountTargets(provider: provider, accounts: accounts, report: report)
            profileTargets(provider: provider, profiles: profiles, report: report)
            diagnostics(provider: provider, report: report, accounts: accounts)
        }
    }

    private func accountTargets(provider: String, accounts: [TargetAccount], report: DashboardReport) -> some View {
        Card(title: "Accounts") {
            // Onboarding always lives in the same top-right "+", empty or not.
            Menu {
                Button("Sign in…") { Task { await store.signIn(provider: provider) } }
                Button("Use existing login") { Task { await store.useExistingLogin(provider: provider) } }
            } label: {
                Image(systemName: "plus")
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .fixedSize()
            .disabled(store.signingInProvider != nil || store.savingExistingLoginProvider != nil)
        } content: {
            if store.signingInProvider == provider {
                signInProgress()
            } else if accounts.isEmpty {
                emptyState("No \(provider.capitalized) accounts yet.")
            }

            ForEach(accounts) { account in
                AccountTargetRow(
                    account: account,
                    active: account.active,
                    switching: store.switchingLocalId == account.id,
                    deleting: store.deletingLocalId == account.id,
                    resetting: store.resettingLocalId == account.id,
                    refreshing: store.refreshingProvider != nil || store.refreshingTargetId == account.id,
                    confirmingDelete: store.confirmingDeleteTargetId == account.id,
                    confirmingReset: store.confirmingResetTargetId == account.id,
                    primary: quotaWindow(account, preferred: .short),
                    secondary: quotaWindow(account, preferred: .weekly),
                    accent: providerColor(account.provider),
                    switchAction: { Task { await store.switchAccount(account) } },
                    requestResetConfirmation: { store.confirmReset(account.id) },
                    cancelResetConfirmation: { store.cancelResetConfirmation() },
                    resetAction: { Task { await store.resetAccountUsageLimit(account) } },
                    refreshAction: { Task { await store.refreshAccount(account) } },
                    requestDeleteConfirmation: { store.confirmDelete(account.id) },
                    cancelDeleteConfirmation: { store.cancelDeleteConfirmation() },
                    deleteAction: { Task { await store.deleteAccount(account) } }
                )
            }
        }
    }

    /// In-flight sign-in: a real, cancellable wait, not a dead spinner. The
    /// backend kills the official-CLI child on Cancel or a 3-minute timeout.
    private func signInProgress() -> some View {
        HStack(spacing: 10) {
            ProgressView().controlSize(.small)
            VStack(alignment: .leading, spacing: 1) {
                Text("Waiting for browser sign-in…")
                    .font(.caption.weight(.medium))
                Text("Finish in your browser, or cancel.")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            Button("Cancel") { store.cancelSignIn() }
                .buttonStyle(CompactCommandButtonStyle())
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func profileTargets(provider: String, profiles: [TargetProfile], report: DashboardReport) -> some View {
        Card(title: "Profiles") {
            Button {
                beginProfileImport(provider: provider)
            } label: {
                if store.importingProfileProvider == provider {
                    ProgressView().controlSize(.small)
                } else {
                    Image(systemName: "plus")
                }
            }
            .buttonStyle(.borderless)
            .fixedSize()
            .disabled(store.importingProfileProvider != nil)
        } content: {
            if profiles.isEmpty {
                emptyState("No \(provider.capitalized) profiles yet.")
            }

            ForEach(profiles) { profile in
                ProfileTargetRow(
                    profile: profile,
                    active: profile.active,
                    switching: store.switchingLocalId == profile.id,
                    deleting: store.deletingLocalId == profile.id,
                    confirmingDelete: store.confirmingDeleteTargetId == profile.id,
                    switchAction: { Task { await store.switchProfile(profile) } },
                    requestDeleteConfirmation: { store.confirmDelete(profile.id) },
                    cancelDeleteConfirmation: { store.cancelDeleteConfirmation() },
                    deleteAction: { Task { await store.deleteProfile(profile) } }
                )
            }
        }
        .onDrop(of: [UTType.fileURL.identifier], isTargeted: nil) { providers in
            handleProfileFileDrop(providers, provider: provider)
        }
    }

    private func diagnostics(provider: String, report: DashboardReport, accounts: [TargetAccount]) -> some View {
        let providerDiagnostics = providerAggregate(provider, in: report)?.diagnostics ?? []
        return Card(title: "Diagnostics") {
            if providerDiagnostics.isEmpty {
                emptyState("No diagnostics for \(provider.capitalized).")
            }

            ForEach(Array(providerDiagnostics.enumerated()), id: \.offset) { _, diagnostic in
                DiagnosticView(diagnostic: diagnostic)
            }
        }
    }

    // Footer is a status strip, not the main action area: a one-line freshness
    // string on the left, then low-frequency exits on the right. Settings
    // already lives in the header gear, so it is not duplicated here.
    private func footer(report: DashboardReport, stale: Bool) -> some View {
        HStack(spacing: 10) {
            Text(footerStatus(report, stale: stale))
                .font(.caption)
                .foregroundStyle(stale ? .orange : .secondary)
                .lineLimit(1)
                .truncationMode(.tail)

            Spacer()

            Menu {
                Button("About Prismux") { onOpenSettings(.about) }
                Button("Copy prismux command") { copyPrismuxCommand() }
                Button("Open Releases") {
                    if let url = URL(string: "https://github.com/hiQianFan/prismux/releases") {
                        NSWorkspace.shared.open(url)
                    }
                }
            } label: {
                Image(systemName: "ellipsis")
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .fixedSize()
            .help("More")
            .accessibilityLabel("More options")

            Button("Quit") { NSApplication.shared.terminate(nil) }
                .buttonStyle(CompactCommandButtonStyle())
        }
        .padding()
    }

    private func footerStatus(_ report: DashboardReport, stale: Bool) -> String {
        let freshness = headerSubtitle(report, stale: stale)
        return cliReady ? "\(freshness) · CLI ready" : "\(freshness) · CLI not configured"
    }

    // Copies the bundled-helper symlink command rather than launching a
    // terminal: copy-and-paste keeps focus and avoids spawning Terminal.app.
    private func copyPrismuxCommand() {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(CliToolStatus.detect().manualCommand, forType: .string)
    }

    @ViewBuilder
    private func emptyState(_ text: String) -> some View {
        Text(text)
            .font(.caption)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func dismissImport() {
        importProvider = nil
        importError = nil
    }

    @ViewBuilder
    private var importProfileOverlay: some View {
        if let provider = importProvider {
            ZStack {
                // Scrim: tapping outside the panel cancels, like a real dialog.
                Color.black.opacity(0.35)
                    .ignoresSafeArea()
                    .onTapGesture { dismissImport() }

                VStack(alignment: .leading, spacing: 12) {
                    HStack(spacing: 8) {
                        Image(systemName: "square.and.arrow.down")
                            .foregroundStyle(providerColor(provider))
                        Text("Import \(provider.capitalized) Profile")
                            .font(.headline)
                        Spacer()
                    }

                    Text("Paste a relay or gateway config, choose a file, or drop one onto the Profiles card.")
                        .font(.caption)
                        .foregroundStyle(.secondary)

                    TextField("Name (optional)", text: $importName)
                        .textFieldStyle(.roundedBorder)

                    TextEditor(text: $importContent)
                        .font(.system(.caption, design: .monospaced))
                        .scrollContentBackground(.hidden)
                        .padding(6)
                        .frame(minHeight: 150)
                        .background(
                            Color.primary.opacity(0.04),
                            in: RoundedRectangle(cornerRadius: 8)
                        )
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(Color.primary.opacity(0.12), lineWidth: 1)
                        )

                    if let importError {
                        Label(importError, systemImage: "exclamationmark.triangle.fill")
                            .font(.caption)
                            .foregroundStyle(.red)
                    }

                    HStack(spacing: 8) {
                        Button {
                            chooseProfileImportFile()
                        } label: {
                            Label("Choose File", systemImage: "folder")
                        }
                        .buttonStyle(CompactCommandButtonStyle())

                        Spacer()

                        Button("Cancel") { dismissImport() }
                            .buttonStyle(CompactCommandButtonStyle())
                            .keyboardShortcut(.cancelAction)

                        Button {
                            submitProfileImport(provider: provider)
                        } label: {
                            Text("Import")
                        }
                        .buttonStyle(ProminentCommandButtonStyle(tint: providerColor(provider)))
                        .keyboardShortcut(.defaultAction)
                        .disabled(importContent.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    }
                }
                .padding(16)
                .background(
                    .regularMaterial,
                    in: RoundedRectangle(cornerRadius: 12)
                )
                .overlay(
                    RoundedRectangle(cornerRadius: 12)
                        .stroke(Color.primary.opacity(0.12), lineWidth: 1)
                )
                .padding(16)
            }
            .transition(.opacity)
        }
    }

    private func beginProfileImport(provider: String) {
        importProvider = provider
        importName = ""
        importContent = ""
        importError = nil
    }

    private func submitProfileImport(provider: String) {
        let content = importContent
        let name = importName.trimmingCharacters(in: .whitespacesAndNewlines)
        Task {
            await store.importProfile(
                provider: provider,
                name: name.isEmpty ? nil : name,
                content: content
            )
            dismissImport()
        }
    }

    private func chooseProfileImportFile() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.canChooseFiles = true
        if panel.runModal() == .OK, let url = panel.url {
            loadProfileImportFile(url)
        }
    }

    private func handleProfileFileDrop(_ providers: [NSItemProvider], provider: String) -> Bool {
        guard let item = providers.first(where: { $0.hasItemConformingToTypeIdentifier(UTType.fileURL.identifier) }) else {
            return false
        }
        item.loadItem(forTypeIdentifier: UTType.fileURL.identifier, options: nil) { value, error in
            DispatchQueue.main.async {
                if let error {
                    importError = error.localizedDescription
                    return
                }
                let url: URL?
                if let value = value as? URL {
                    url = value
                } else if let data = value as? Data {
                    url = URL(dataRepresentation: data, relativeTo: nil)
                } else {
                    url = nil
                }
                guard let url else {
                    importError = "Dropped item was not a readable file."
                    return
                }
                beginProfileImport(provider: provider)
                loadProfileImportFile(url)
            }
        }
        return true
    }

    private func loadProfileImportFile(_ url: URL) {
        do {
            let values = try url.resourceValues(forKeys: [.fileSizeKey])
            if let size = values.fileSize, size > 262_144 {
                throw ProfileImportInputError("Profile config file must be 256 KiB or smaller.")
            }
            importContent = try String(contentsOf: url, encoding: .utf8)
            importError = nil
        } catch {
            importError = error.localizedDescription
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

    private func accounts(for provider: String, in report: DashboardReport) -> [TargetAccount] {
        report.accounts.accounts
            .filter { $0.provider == provider }
            .sorted { $0.displayNumber == $1.displayNumber ? $0.localId < $1.localId : $0.displayNumber < $1.displayNumber }
    }

    private func profiles(for provider: String, in report: DashboardReport) -> [TargetProfile] {
        report.accounts.profiles
            .filter { $0.provider == provider }
            .sorted { $0.displayNumber == $1.displayNumber ? $0.name < $1.name : $0.displayNumber < $1.displayNumber }
    }

    private func providerView(_ provider: String, in report: DashboardReport) -> ProviderView? {
        report.providerViews?.first { $0.provider == provider }
    }

    private func providerAggregate(_ provider: String, in report: DashboardReport) -> ProviderAggregateView? {
        report.aggregate.providerAggregates.first { $0.providerId == provider }
    }

    private func providerAttentionCount(_ report: DashboardReport) -> Int {
        (report.providerViews ?? []).filter(isProviderAttention).count
    }

    private func isProviderAttention(_ view: ProviderView) -> Bool {
        view.statusTone == "warning" || view.statusTone == "danger"
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

    private func activeAccount(_ accounts: [TargetAccount]) -> TargetAccount? {
        accounts.first(where: \.active)
    }

    private func activeProfile(_ profiles: [TargetProfile]) -> TargetProfile? {
        profiles.first(where: \.active)
    }

    private func activeTargetLabel(accounts: [TargetAccount], profiles: [TargetProfile]) -> String {
        if let account = activeAccount(accounts) {
            return hidePersonalIdentifiers ? "#\(account.displayNumber) Account" : account.shortLabel
        }
        if let profile = activeProfile(profiles) {
            return hidePersonalIdentifiers ? "#\(profile.displayNumber) Profile" : profile.displayLabel
        }
        return "-"
    }

    private func activeTargetLabel(account: TargetAccount?, profile: TargetProfile?) -> String? {
        if let account {
            return hidePersonalIdentifiers ? "#\(account.displayNumber) Account" : account.shortLabel
        }
        if let profile {
            return hidePersonalIdentifiers ? "#\(profile.displayNumber) Profile" : profile.displayLabel
        }
        return nil
    }

    private func providerUpdatedLabel(accounts: [TargetAccount], fallback: UInt64) -> String {
        let refreshed = accounts.compactMap { $0.quota?.refreshedAtUnix }.max()
        if let refreshed {
            return timeAgo(refreshed)
        }
        return timeAgo(fallback)
    }

    private func providerSecondaryText(accounts: [TargetAccount], profiles: [TargetProfile]) -> String {
        if accounts.isEmpty, profiles.isEmpty {
            return "No accounts"
        }
        let accountText = accounts.isEmpty ? "No accounts" : "\(accounts.count) account\(accounts.count == 1 ? "" : "s")"
        let profileText = profiles.isEmpty ? "No profiles" : "\(profiles.count) profile\(profiles.count == 1 ? "" : "s")"
        return "\(accountText) · \(profileText)"
    }

    private enum WindowPreference {
        case short
        case weekly
    }

    private func quotaWindow(_ account: TargetAccount, preferred: WindowPreference) -> QuotaWindow? {
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

/// Filled, tinted primary action. Used for the onboarding CTA so "Sign in" /
/// "Import" reads as the one obvious next step, not another row verb.
private struct ProminentCommandButtonStyle: ButtonStyle {
    var tint: Color = .accentColor

    func makeBody(configuration: Configuration) -> some View {
        PressableChrome(
            configuration: configuration,
            tint: tint,
            horizontalPadding: 12,
            verticalPadding: 6,
            minWidth: 0,
            minHeight: 0,
            cornerRadius: 7,
            filled: true
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
    var filled: Bool = false

    var body: some View {
        configuration.label
            .padding(.horizontal, horizontalPadding)
            .padding(.vertical, verticalPadding)
            .frame(minWidth: minWidth, minHeight: minHeight)
            .foregroundStyle(filled ? Color.white : tint)
            .background(
                filled ? tint.opacity(configuration.isPressed ? 0.85 : 1) : tint.opacity(backgroundOpacity),
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

private struct Card<Header: View, Content: View>: View {
    @Environment(\.colorScheme) private var colorScheme
    private let title: String?
    private let accessory: Header
    private let content: Content

    init(
        title: String? = nil,
        @ViewBuilder accessory: () -> Header = { EmptyView() },
        @ViewBuilder content: () -> Content
    ) {
        self.title = title
        self.accessory = accessory()
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if let title {
                HStack(spacing: 8) {
                    Text(title)
                        .font(.headline)
                    Spacer(minLength: 0)
                    accessory
                }
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

private struct ProfileImportInputError: LocalizedError {
    let message: String

    init(_ message: String) {
        self.message = message
    }

    var errorDescription: String? { message }
}
