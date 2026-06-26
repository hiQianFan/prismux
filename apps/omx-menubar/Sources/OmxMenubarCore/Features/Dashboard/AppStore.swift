import Foundation

@MainActor
public final class AppStore: ObservableObject {
    enum State {
        case loading
        case ready(DashboardReport, stale: Bool)
        case failed(lastGood: DashboardReport?, message: String)
    }

    @Published private(set) var state: State = .loading
    @Published private(set) var switchingLocalId: String?
    @Published private(set) var deletingLocalId: String?
    @Published private(set) var confirmingDeleteTargetId: String?
    @Published private(set) var refreshingProvider: String?
    @Published private(set) var statusMessage: String?
    @Published private(set) var statusKind: String?
    @Published var selectedProvider: String?

    var trayTitle: String {
        switch state {
        case .loading:
            return "OpenMux"
        case .failed(let lastGood, _):
            guard let report = lastGood else { return "OpenMux !" }
            return "\(aggregateTraySignal(report)) stale"
        case .ready(let report, let stale):
            let signal = aggregateTraySignal(report)
            return stale ? "\(signal) stale" : signal
        }
    }

    private let backend: BackendClient
    private var generation: UInt64 = 0
    private var lastGood: DashboardReport?

    public init(backend: RustBackendClient) {
        self.backend = backend
    }

    init(testBackend backend: BackendClient) {
        self.backend = backend
    }

    public func load() async {
        await request(.dashboard(provider: nil))
    }

    func refresh(provider: String? = nil, kind: String) async {
        guard refreshingProvider == nil else { return }
        guard let target = provider else {
            await refreshAll(providers: currentProviders, kind: kind)
            return
        }
        guard !target.isEmpty else {
            await load()
            return
        }
        refreshingProvider = target
        await request(.refresh(provider: target, kind: kind))
        refreshingProvider = nil
    }

    func refreshAll(providers: [String], kind: String) async {
        guard refreshingProvider == nil else { return }
        if providers.isEmpty {
            await load()
            return
        }
        for provider in providers {
            refreshingProvider = provider
            await request(.refresh(provider: provider, kind: kind))
        }
        refreshingProvider = nil
        await load()
    }

    func switchAccount(_ account: MenubarAccount) async {
        guard switchingLocalId == nil else { return }
        switchingLocalId = account.id
        statusMessage = nil
        statusKind = nil
        await request(.switchTarget(provider: account.provider, targetKind: account.targetKind, localId: account.localId))
        switchingLocalId = nil
    }

    func switchProfile(_ profile: MenubarProfile) async {
        guard switchingLocalId == nil else { return }
        switchingLocalId = profile.id
        statusMessage = nil
        statusKind = nil
        await request(.switchTarget(provider: profile.provider, targetKind: profile.targetKind, localId: profile.localId))
        switchingLocalId = nil
    }

    func deleteAccount(_ account: MenubarAccount) async {
        guard deletingLocalId == nil else { return }
        confirmingDeleteTargetId = nil
        deletingLocalId = account.id
        await request(.removeTarget(provider: account.provider, targetKind: account.targetKind, localId: account.localId))
        deletingLocalId = nil
    }

    func deleteProfile(_ profile: MenubarProfile) async {
        guard deletingLocalId == nil else { return }
        confirmingDeleteTargetId = nil
        deletingLocalId = profile.id
        await request(.removeTarget(provider: profile.provider, targetKind: profile.targetKind, localId: profile.localId))
        deletingLocalId = nil
    }

    func confirmDelete(_ targetId: String) {
        confirmingDeleteTargetId = targetId
    }

    func cancelDeleteConfirmation() {
        confirmingDeleteTargetId = nil
    }

    private func request(_ payload: Payload) async {
        generation += 1
        let currentGeneration = generation
        do {
            let envelope = try await backend.call(BackendRequest(
                schemaVersion: 1,
                op: opName(payload),
                payload: payload,
                requestId: UUID().uuidString
            ))
            guard currentGeneration == generation else { return }
            if let report = envelope.data?.dashboard {
                lastGood = report
                statusMessage = envelope.data?.operation?.message
                statusKind = envelope.data?.operation?.status
                state = .ready(report, stale: false)
            } else {
                statusMessage = "Backend returned no dashboard."
                statusKind = "failed"
            }
        } catch {
            guard currentGeneration == generation else { return }
            let message = userFacingMessage(error)
            statusMessage = message
            statusKind = "failed"
            state = .failed(lastGood: lastGood, message: message)
        }
    }

    private func userFacingMessage(_ error: Error) -> String {
        if let localized = error as? LocalizedError,
           let description = localized.errorDescription,
           !description.isEmpty {
            return description
        }
        return error.localizedDescription
    }

    private var currentProviders: [String] {
        switch state {
        case .ready(let report, _):
            return providerNames(from: report)
        case .failed(let lastGood, _):
            return lastGood.map(providerNames(from:)) ?? []
        case .loading:
            return []
        }
    }

    private func aggregateTraySignal(_ report: DashboardReport) -> String {
        let accounts = report.accounts.accounts
        if accounts.isEmpty {
            return "OpenMux -"
        }

        if let urgent = accounts
            .compactMap({ account -> (String, UInt32)? in
                guard let remaining = account.quota?.primaryWindow?.remainingPercentX100 else {
                    return nil
                }
                return (account.provider.capitalized, remaining)
            })
            .min(by: { $0.1 < $1.1 })
        {
            return "\(urgent.0) \(Int(urgent.1) / 100)%"
        }

        let troubled = accounts.filter { account in
            account.status != "healthy" || account.diagnostic != nil
        }.count
        if troubled > 0 {
            return "\(troubled) alerts"
        }

        let providerCount = providerNames(from: report).count
        return providerCount == 1 ? "1 provider" : "\(providerCount) providers"
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
        case .refresh: return "refresh"
        case .switchTarget: return "switch"
        case .removeTarget: return "remove"
        }
    }
}
