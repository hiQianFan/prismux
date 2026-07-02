import AppKit
import Foundation
import ServiceManagement

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
    @Published var launchAtLogin: Bool = SMAppService.mainApp.status == .enabled
    @Published var cliStatus: CliToolStatus = .init()

    private let backend: BackendClient
    private let refreshCadenceKey = "dev.prismux.menubar.backgroundRefreshCadence"
    private let privacyKey = "dev.prismux.menubar.hidePersonalIdentifiers"

    init(backend: BackendClient = RustBackendClient()) {
        self.backend = backend
    }

    func load() async {
        loading = true
        errorMessage = nil
        defer { loading = false }
        launchAtLogin = SMAppService.mainApp.status == .enabled
        cliStatus = CliToolStatus.detect()
        do {
            async let settingsEnvelope = backend.call(BackendRequest(schemaVersion: 2, op: "settings_view", payload: .settingsView, requestId: nil))
            async let aboutEnvelope = backend.call(BackendRequest(schemaVersion: 2, op: "about_view", payload: .aboutView, requestId: nil))
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

    func setLaunchAtLogin(_ enabled: Bool) {
        do {
            if enabled {
                try SMAppService.mainApp.register()
            } else {
                try SMAppService.mainApp.unregister()
            }
        } catch {
            errorMessage = error.localizedDescription
        }
        // Reflect the real registration state rather than the requested one.
        launchAtLogin = SMAppService.mainApp.status == .enabled
    }

    func copyCliCommand() {
        copy(cliStatus.manualCommand)
        supportStatus = "prismux command copied"
    }

    func copyPathCommand() {
        guard let command = cliStatus.pathCommand else { return }
        copy(command)
        supportStatus = "PATH command copied"
    }

    func enableCliCommand() {
        do {
            try CliToolStatus.installSymlink()
            cliStatus = CliToolStatus.detect()
            supportStatus = "prismux command enabled"
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func openLink(_ url: String) {
        guard let parsed = URL(string: url) else { return }
        NSWorkspace.shared.open(parsed)
    }

    func copyVersionInfo() {
        guard let about else { return }
        let text = """
        Prismux \(about.appVersion)
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
                schemaVersion: 2,
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
                    schemaVersion: 2,
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

/// Read-only inspection of the bundled `prismux` helper and how a Terminal `prismux`
/// command resolves. No mutation here — `Enable prismux command` symlink creation
/// is a separate, explicit follow-up; this only reports state and hands back
/// copyable commands.
struct CliToolStatus: Sendable {
    enum Resolution: Sendable {
        case ready          // PATH prismux resolves to the bundled helper
        case notConfigured  // no prismux on PATH
        case differentFound // some other prismux on PATH
    }

    var bundledPath: String?
    var helperVersion: String?
    var helperAvailable: Bool = false
    var resolution: Resolution = .notConfigured
    var foundPath: String?
    var foundVersion: String?
    var proxySource: String = "None"

    /// `~/.local/bin` symlink command, copyable when not yet configured.
    var manualCommand: String {
        let target = bundledPath ?? "Prismux.app/Contents/SharedSupport/bin/prismux"
        let quotedTarget = Self.shellQuote(target)
        return """
        mkdir -p "$HOME/.local/bin"
        if [ -L "$HOME/.local/bin/prismux" ] || [ ! -e "$HOME/.local/bin/prismux" ]; then
          ln -sfn \(quotedTarget) "$HOME/.local/bin/prismux"
        else
          echo "$HOME/.local/bin/prismux already exists; remove it manually first" >&2
        fi
        """
    }

    /// Shown only when `~/.local/bin` is not already on PATH.
    var pathCommand: String? {
        let localBin = (NSHomeDirectory() as NSString).appendingPathComponent(".local/bin")
        let path = ProcessInfo.processInfo.environment["PATH"] ?? ""
        guard !path.split(separator: ":").contains(where: { $0 == Substring(localBin) }) else { return nil }
        return #"echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.zshrc""#
    }

    var statusText: String {
        switch resolution {
        case .ready: "Ready"
        case .notConfigured: "Not configured"
        case .differentFound: "Different prismux found"
        }
    }

    var statusTone: String {
        switch resolution {
        case .ready: "success"
        case .notConfigured: "muted"
        case .differentFound: "warning"
        }
    }

    static func detect() -> CliToolStatus {
        var status = CliToolStatus()
        // Bundled helper lives in Contents/SharedSupport/bin (macOS convention for
        // user-facing CLIs), kept out of Contents/MacOS to avoid a case-insensitive
        // filename collision with the "Prismux" app executable.
        if let helper = Bundle.main.sharedSupportURL?
            .appendingPathComponent("bin/prismux") {
            status.bundledPath = helper.path
            status.helperAvailable = FileManager.default.isExecutableFile(atPath: helper.path)
            status.helperVersion = Self.version(of: helper.path)
        }
        status.proxySource = Self.detectProxySource()

        // Resolve `prismux` on PATH without spawning a shell.
        let path = ProcessInfo.processInfo.environment["PATH"] ?? ""
        for dir in path.split(separator: ":") {
            let candidate = (String(dir) as NSString).appendingPathComponent("prismux")
            guard FileManager.default.isExecutableFile(atPath: candidate) else { continue }
            status.foundPath = candidate
            status.foundVersion = Self.version(of: candidate)
            let sameBinary = status.bundledPath.map { resolvedPath(candidate) == resolvedPath($0) } ?? false
            let sameVersion = status.helperVersion != nil && status.helperVersion == status.foundVersion
            if sameBinary || sameVersion {
                status.resolution = .ready
            } else {
                status.resolution = .differentFound
            }
            return status
        }
        status.resolution = .notConfigured
        return status
    }

    static func installSymlink() throws {
        let current = detect()
        guard let bundled = current.bundledPath, current.helperAvailable else {
            throw NSError(domain: "PrismuxCliTool", code: 1, userInfo: [
                NSLocalizedDescriptionKey: "Bundled prismux helper is unavailable."
            ])
        }

        let fileManager = FileManager.default
        let localBin = (NSHomeDirectory() as NSString).appendingPathComponent(".local/bin")
        let installPath = (localBin as NSString).appendingPathComponent("prismux")
        try fileManager.createDirectory(atPath: localBin, withIntermediateDirectories: true)

        if (try? fileManager.destinationOfSymbolicLink(atPath: installPath)) != nil {
            try fileManager.removeItem(atPath: installPath)
        } else if fileManager.fileExists(atPath: installPath) {
            throw NSError(domain: "PrismuxCliTool", code: 2, userInfo: [
                NSLocalizedDescriptionKey: "\(installPath) already exists and is not a symlink."
            ])
        }

        try fileManager.createSymbolicLink(atPath: installPath, withDestinationPath: bundled)
    }

    private static func detectProxySource() -> String {
        let env = ProcessInfo.processInfo.environment
        for key in ["PRISMUX_HTTPS_PROXY", "HTTPS_PROXY", "ALL_PROXY"] {
            if let value = env[key], !value.isEmpty {
                return "Environment \(key)"
            }
        }
        return "None"
    }

    private static func version(of executable: String) -> String? {
        guard FileManager.default.isExecutableFile(atPath: executable) else { return nil }
        let process = Process()
        process.executableURL = URL(fileURLWithPath: executable)
        process.arguments = ["--version"]
        let output = Pipe()
        process.standardOutput = output
        process.standardError = Pipe()
        do {
            try process.run()
            process.waitUntilExit()
            guard process.terminationStatus == 0 else { return nil }
            let data = output.fileHandleForReading.readDataToEndOfFile()
            let parts = String(decoding: data, as: UTF8.self).split(whereSeparator: \.isWhitespace)
            guard parts.count >= 2 else { return nil }
            return String(parts[1])
        } catch {
            return nil
        }
    }

    private static func resolvedPath(_ path: String) -> String {
        URL(fileURLWithPath: path).resolvingSymlinksInPath().path
    }

    private static func shellQuote(_ value: String) -> String {
        "'" + value.replacingOccurrences(of: "'", with: "'\\''") + "'"
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
