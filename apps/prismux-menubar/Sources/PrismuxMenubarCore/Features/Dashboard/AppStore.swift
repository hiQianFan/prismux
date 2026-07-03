import Foundation

@MainActor
public final class AppStore: ObservableObject {
    @Published private(set) var state: MenubarState = .loading
    @Published private(set) var switchingLocalId: String?
    @Published private(set) var deletingLocalId: String?
    @Published private(set) var confirmingDeleteTargetId: String?
    @Published private(set) var resettingLocalId: String?
    @Published private(set) var confirmingResetTargetId: String?
    @Published private(set) var refreshingProvider: String?
    @Published private(set) var refreshingTargetId: String?
    @Published private(set) var signingInProvider: String?
    @Published private(set) var savingExistingLoginProvider: String?
    @Published private(set) var importingProfileProvider: String?
    @Published private(set) var operationNotice: OperationNotice?
    @Published var selectedProvider: String?

    private let backend: BackendClient
    private var generation: UInt64 = 0
    private var lastGood: DashboardReport?
    /// Tail of the write-command chain. Every state-mutating command links onto
    /// it so the order the UI issues commands in is exactly the order the
    /// backend executes them. See `runSerial`.
    private var writeTail: Task<Void, Never>?

    public init(backend: RustBackendClient) {
        self.backend = backend
    }

    init(testBackend backend: BackendClient) {
        self.backend = backend
    }

    /// Serialize state-mutating commands (sign in / import / switch / delete /
    /// reset). They are dispatched from the UI as independent `Task`s, so their
    /// detached FFI calls could otherwise reach the backend's operation lock out
    /// of order; the UI would then apply whichever response returned last, which
    /// is not necessarily the write the backend applied last. Chaining each
    /// command onto the previous one makes UI-issue-order == backend-apply-order,
    /// so the newest response always reflects the newest write.
    ///
    /// Reads (`load`/`refresh`) deliberately stay OFF this chain: they are
    /// concurrent and idempotent, and `generation` keeps only the newest read.
    /// Writes always apply their own response; a later refresh must not hide a
    /// completed user action.
    private func runSerial(_ work: @escaping @MainActor () async -> Void) async {
        let prior = writeTail
        let task = Task { @MainActor in
            await prior?.value
            await work()
        }
        writeTail = task
        await task.value
    }

    public func load() async {
        await request(.dashboard(provider: nil))
    }

    func refresh(provider: String? = nil, kind: String) async {
        guard !refreshInProgress else { return }
        // Skip background refreshes while a sign-in/import is in flight: let the
        // onboarding command's own fresh response land instead of racing it with
        // a redundant read. This avoids needless work and UI churn.
        guard !onboardingInProgress else { return }
        guard let target = provider else {
            await refreshAll(providers: currentProviders, kind: kind)
            return
        }
        guard !target.isEmpty else {
            await load()
            return
        }
        refreshingProvider = target
        await request(.refresh(provider: target, kind: kind, targetKind: nil, localId: nil))
        refreshingProvider = nil
    }

    func refreshAll(providers: [String], kind: String) async {
        guard !refreshInProgress else { return }
        guard !onboardingInProgress else { return }
        if providers.isEmpty {
            await load()
            return
        }
        for provider in providers {
            refreshingProvider = provider
            await request(.refresh(provider: provider, kind: kind, targetKind: nil, localId: nil))
        }
        refreshingProvider = nil
    }

    func refreshAccount(_ account: TargetAccount, kind: String = "interactive") async {
        guard !refreshInProgress else { return }
        refreshingTargetId = account.id
        await request(.refresh(
            provider: account.provider,
            kind: kind,
            targetKind: account.targetKind,
            localId: account.localId
        ))
        refreshingTargetId = nil
    }

    private var refreshInProgress: Bool {
        refreshingProvider != nil || refreshingTargetId != nil
    }

    private var onboardingInProgress: Bool {
        signingInProvider != nil
            || savingExistingLoginProvider != nil
            || importingProfileProvider != nil
    }

    func switchAccount(_ account: TargetAccount) async {
        await runSerial { [weak self] in
            guard let self, self.switchingLocalId == nil else { return }
            self.switchingLocalId = account.id
            await self.request(.switchTarget(provider: account.provider, targetKind: account.targetKind, localId: account.localId))
            self.switchingLocalId = nil
        }
    }

    func switchProfile(_ profile: TargetProfile) async {
        await runSerial { [weak self] in
            guard let self, self.switchingLocalId == nil else { return }
            self.switchingLocalId = profile.id
            await self.request(.switchTarget(provider: profile.provider, targetKind: profile.targetKind, localId: profile.localId))
            self.switchingLocalId = nil
        }
    }

    func deleteAccount(_ account: TargetAccount) async {
        await runSerial { [weak self] in
            guard let self, self.deletingLocalId == nil else { return }
            self.confirmingDeleteTargetId = nil
            self.deletingLocalId = account.id
            await self.request(.removeTarget(provider: account.provider, targetKind: account.targetKind, localId: account.localId))
            self.deletingLocalId = nil
        }
    }

    func deleteProfile(_ profile: TargetProfile) async {
        await runSerial { [weak self] in
            guard let self, self.deletingLocalId == nil else { return }
            self.confirmingDeleteTargetId = nil
            self.deletingLocalId = profile.id
            await self.request(.removeTarget(provider: profile.provider, targetKind: profile.targetKind, localId: profile.localId))
            self.deletingLocalId = nil
        }
    }

    func resetAccountUsageLimit(_ account: TargetAccount) async {
        await runSerial { [weak self] in
            guard let self, self.resettingLocalId == nil else { return }
            self.confirmingResetTargetId = nil
            self.resettingLocalId = account.id
            await self.request(.consumeResetCredit(
                provider: account.provider,
                targetKind: account.targetKind,
                localId: account.localId,
                idempotencyKey: UUID().uuidString
            ))
            self.resettingLocalId = nil
        }
    }

    func signIn(provider: String) async {
        await runSerial { [weak self] in
            guard let self, !self.onboardingInProgress else { return }
            self.signingInProvider = provider
            await self.request(.login(
                provider: provider,
                alias: nil,
                activate: false,
                deviceAuth: false
            ))
            self.signingInProvider = nil
        }
    }

    /// Cancel an in-flight `signIn`. Fire-and-forget on its own backend call so
    /// it reaches the FFI while the login task is parked holding the operation
    /// lock; it only flips the backend cancel flag. Does not go through
    /// `request(_:)` — that would bump generation and clobber dashboard state.
    func cancelSignIn() {
        guard signingInProvider != nil else { return }
        Task.detached { [backend] in
            _ = try? await backend.call(BackendRequest(
                schemaVersion: 2,
                op: "cancel_login",
                payload: .cancelLogin,
                requestId: UUID().uuidString
            ))
        }
    }

    func useExistingLogin(provider: String) async {
        await runSerial { [weak self] in
            guard let self, !self.onboardingInProgress else { return }
            self.savingExistingLoginProvider = provider
            await self.request(.saveExistingLogin(provider: provider, alias: nil))
            self.savingExistingLoginProvider = nil
        }
    }

    func importProfile(provider: String, name: String?, content: String) async {
        await runSerial { [weak self] in
            guard let self, !self.onboardingInProgress else { return }
            self.importingProfileProvider = provider
            await self.request(.importProfile(
                provider: provider,
                name: name,
                content: content
            ))
            self.importingProfileProvider = nil
        }
    }

    func confirmDelete(_ targetId: String) {
        confirmingDeleteTargetId = targetId
    }

    func cancelDeleteConfirmation() {
        confirmingDeleteTargetId = nil
    }

    func confirmReset(_ targetId: String) {
        confirmingResetTargetId = targetId
    }

    func cancelResetConfirmation() {
        confirmingResetTargetId = nil
    }

    private func request(_ payload: Payload) async {
        generation += 1
        let currentGeneration = generation
        let discardIfStale = isRead(payload)
        do {
            let envelope = try await backend.call(BackendRequest(
                schemaVersion: 2,
                op: opName(payload),
                payload: payload,
                requestId: UUID().uuidString
            ))
            guard !discardIfStale || currentGeneration == generation else { return }
            if let operation = envelope.data?.operation {
                operationNotice = OperationNotice(operation: operation)
            }
            if let report = envelope.data?.dashboard {
                lastGood = report
                state = .ready(report, stale: envelope.dataStale == true || envelope.servedFromSnapshot == true)
            } else {
                state = .failed(lastGood: lastGood, message: "Backend returned no dashboard.")
            }
        } catch {
            guard !discardIfStale || currentGeneration == generation else { return }
            let message = userFacingMessage(error)
            state = .backendUnavailable(lastGood: lastGood, message: message)
        }
    }

    private func isRead(_ payload: Payload) -> Bool {
        switch payload {
        case .dashboard, .accounts, .refresh, .compatibility, .settingsView, .aboutView, .supportReport:
            return true
        case .switchTarget, .removeTarget, .consumeResetCredit, .login, .saveExistingLogin, .importProfile, .updateSettings, .cancelLogin:
            return false
        }
    }

    private func userFacingMessage(_ error: Error) -> String {
        if let localized = error as? LocalizedError,
           let description = localized.errorDescription,
           !description.isEmpty {
            if description.contains("failed to run"), description.contains("login") {
                if description.contains("codex") {
                    return "Codex CLI was not found. Install Codex CLI, then try Sign in again."
                }
                if description.contains("claude") {
                    return "Claude Code CLI was not found. Install Claude Code, then try Sign in again."
                }
            }
            return description
        }
        return error.localizedDescription
    }

    private var currentProviders: [String] {
        switch state {
        case .ready(let report, _):
            return providerNames(from: report)
        case .failed(let lastGood, _), .backendUnavailable(let lastGood, _):
            return lastGood.map(providerNames(from:)) ?? []
        case .loading, .upgradeRequired:
            return []
        }
    }

    private func providerNames(from report: DashboardReport) -> [String] {
        let declared = report.accounts.providers
        if !declared.isEmpty {
            return declared
        }
        return Array(Set(report.accounts.accounts.map(\.provider))).sorted()
    }

    private func opName(_ payload: Payload) -> String {
        switch payload {
        case .dashboard: return "dashboard"
        case .accounts: return "accounts"
        case .compatibility: return "compatibility"
        case .settingsView: return "settings_view"
        case .updateSettings: return "update_settings"
        case .aboutView: return "about_view"
        case .supportReport: return "support_report"
        case .refresh: return "refresh"
        case .switchTarget: return "switch"
        case .removeTarget: return "remove"
        case .consumeResetCredit: return "consume_reset_credit"
        case .login: return "login"
        case .saveExistingLogin: return "save_existing_login"
        case .importProfile: return "import_profile"
        case .cancelLogin: return "cancel_login"
        }
    }
}

struct OperationNotice: Equatable {
    let title: String
    let message: String
    let severity: StatusBannerProps.Severity

    init(operation: OperationResult) {
        self.message = operation.message
        switch operation.status {
        case "failed":
            self.title = "Operation failed"
            self.severity = .error
        case "skipped":
            self.title = "Operation skipped"
            self.severity = .warning
        default:
            self.title = "Operation complete"
            self.severity = .info
        }
    }
}
