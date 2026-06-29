import AppKit
import Foundation

enum MenubarSettingsTab: String, CaseIterable, Identifiable {
    case general
    case providers
    case about

    var id: String { rawValue }

    var title: String {
        switch self {
        case .general: "General"
        case .providers: "Providers"
        case .about: "About"
        }
    }

    var icon: String {
        switch self {
        case .general: "gearshape"
        case .providers: "square.stack.3d.up"
        case .about: "info.circle"
        }
    }
}

@MainActor
final class MenubarSettingsStore: ObservableObject {
    @Published var selectedTab: MenubarSettingsTab = .general
    @Published var settings: SettingsView?
    @Published var about: AboutView?
    @Published var loading = false
    @Published var saving = false
    @Published var errorMessage: String?
    @Published var supportStatus: String?

    private let backend: BackendClient
    private let refreshCadenceKey = "dev.openmux.menubar.backgroundRefreshCadence"
    private let privacyKey = "dev.openmux.menubar.hidePersonalIdentifiers"

    init(backend: BackendClient = RustBackendClient()) {
        self.backend = backend
    }

    func load() async {
        loading = true
        errorMessage = nil
        defer { loading = false }
        do {
            async let settingsEnvelope = backend.call(BackendRequest(schemaVersion: 1, op: "settings_view", payload: .settingsView, requestId: nil))
            async let aboutEnvelope = backend.call(BackendRequest(schemaVersion: 1, op: "about_view", payload: .aboutView, requestId: nil))
            let settingsResult = try await settingsEnvelope
            let aboutResult = try await aboutEnvelope
            let loadedSettings = settingsResult.data?.settings
            let loadedAbout = aboutResult.data?.about
            settings = loadedSettings
            about = loadedAbout
            if let loadedSettings {
                syncLocalPreferences(loadedSettings)
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func updateRefreshCadence(_ seconds: UInt64) {
        guard var settings else { return }
        settings.general.refreshCadenceSeconds = seconds
        save(settings)
    }

    func updatePrivacy(_ hidePersonalIdentifiers: Bool) {
        guard var settings else { return }
        settings.privacy.hidePersonalIdentifiers = hidePersonalIdentifiers
        save(settings)
    }

    func updateProviderEnabled(_ provider: ProviderSettings, enabled: Bool) {
        mutateProvider(provider.provider) { $0.enabled = enabled }
    }

    func updateProviderSource(_ provider: ProviderSettings, sourcePreference: SourcePreference) {
        mutateProvider(provider.provider) { $0.sourcePreference = sourcePreference }
    }

    func copyVersionInfo() {
        guard let about else { return }
        let text = """
        OpenMux \(about.appVersion)
        Control plane schema: \(about.controlPlaneSchemaVersion)
        State schema: \(about.stateSchemaVersion)
        Settings schema: \(about.settingsSchemaVersion)
        Runtime: \(about.runtime.mode)
        State root: \(about.stateRoot.display)
        """
        copy(text)
        supportStatus = "Version info copied"
    }

    func copySupportReport() async {
        supportStatus = nil
        do {
            let envelope = try await backend.call(BackendRequest(
                schemaVersion: 1,
                op: "support_report",
                payload: .supportReport(includeDebugSummary: false, recentDiagnostics: []),
                requestId: nil
            ))
            guard let report = envelope.data?.support else { return }
            let encoder = JSONEncoder()
            encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
            let data = try encoder.encode(SupportReportClipboard(report: report))
            copy(String(decoding: data, as: UTF8.self))
            supportStatus = "Redacted support report copied"
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func reveal(_ path: AboutPath) {
        guard let revealPath = path.revealPath else { return }
        NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: revealPath)])
    }

    private func mutateProvider(_ providerID: String, mutation: (inout ProviderSettings) -> Void) {
        guard var settings, let index = settings.providers.firstIndex(where: { $0.provider == providerID }) else { return }
        mutation(&settings.providers[index])
        save(settings)
    }

    private func save(_ nextSettings: SettingsView) {
        settings = nextSettings
        syncLocalPreferences(nextSettings)
        saving = true
        errorMessage = nil
        Task {
            do {
                let envelope = try await backend.call(BackendRequest(
                    schemaVersion: 1,
                    op: "update_settings",
                    payload: .updateSettings(nextSettings),
                    requestId: nil
                ))
                if let saved = envelope.data?.settings {
                    settings = saved
                    syncLocalPreferences(saved)
                }
            } catch {
                errorMessage = error.localizedDescription
                await load()
            }
            saving = false
        }
    }

    private func syncLocalPreferences(_ settings: SettingsView) {
        UserDefaults.standard.set(Int(settings.general.refreshCadenceSeconds), forKey: refreshCadenceKey)
        UserDefaults.standard.set(settings.privacy.hidePersonalIdentifiers, forKey: privacyKey)
    }

    private func copy(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }
}

private struct SupportReportClipboard: Encodable {
    let schemaVersion: UInt32
    let appVersion: String
    let controlPlaneSchemaVersion: UInt32
    let stateSchemaVersion: UInt32
    let settingsSchemaVersion: UInt32
    let redactionStatus: String
    let diagnostics: [SupportDiagnosticClipboard]

    init(report: SupportReport) {
        schemaVersion = report.schemaVersion
        appVersion = report.appVersion
        controlPlaneSchemaVersion = report.controlPlaneSchemaVersion
        stateSchemaVersion = report.stateSchemaVersion
        settingsSchemaVersion = report.settingsSchemaVersion
        redactionStatus = report.redactionStatus
        diagnostics = report.diagnostics.map(SupportDiagnosticClipboard.init)
    }

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case appVersion = "app_version"
        case controlPlaneSchemaVersion = "control_plane_schema_version"
        case stateSchemaVersion = "state_schema_version"
        case settingsSchemaVersion = "settings_schema_version"
        case redactionStatus = "redaction_status"
        case diagnostics
    }
}

private struct SupportDiagnosticClipboard: Encodable {
    let code: String
    let severity: String
    let userMessage: String
    let recoveryAction: String?
    let source: String
    let redactionStatus: String

    init(diagnostic: SupportDiagnostic) {
        code = diagnostic.code
        severity = diagnostic.severity
        userMessage = diagnostic.userMessage
        recoveryAction = diagnostic.recoveryAction
        source = diagnostic.source
        redactionStatus = diagnostic.redactionStatus
    }

    enum CodingKeys: String, CodingKey {
        case code
        case severity
        case userMessage = "user_message"
        case recoveryAction = "recovery_action"
        case source
        case redactionStatus = "redaction_status"
    }
}
