import Foundation

protocol BackendClient: Sendable {
    func call(_ request: BackendRequest) async throws -> BackendEnvelope
}

struct BackendRequest: Encodable, Sendable {
    let schemaVersion: Int
    let op: String
    let payload: Payload
    let requestId: String?

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case op
        case payload
        case requestId = "request_id"
    }
}

enum Payload: Encodable, Sendable {
    case dashboard(provider: String?, usagePeriod: UsagePeriod?)
    case accounts(provider: String?)
    case compatibility
    case settingsView
    case updateSettings(SettingsView)
    case aboutView
    case supportReport(includeDebugSummary: Bool, recentDiagnostics: [String])
    case refresh(provider: String, kind: String, targetKind: String?, localId: String?, usagePeriod: UsagePeriod?)
    case switchTarget(provider: String, targetKind: String, localId: String, usagePeriod: UsagePeriod?)
    case removeTarget(provider: String, targetKind: String, localId: String, usagePeriod: UsagePeriod?)
    case consumeResetCredit(provider: String, targetKind: String, localId: String, idempotencyKey: String, usagePeriod: UsagePeriod?)
    case login(provider: String, alias: String?, activate: Bool, deviceAuth: Bool, usagePeriod: UsagePeriod?)
    case saveExistingLogin(provider: String, alias: String?, usagePeriod: UsagePeriod?)
    case importProfile(provider: String, name: String?, content: String, usagePeriod: UsagePeriod?)
    case cancelLogin

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .dashboard(let provider, let usagePeriod):
            try container.encodeIfPresent(provider, forKey: .provider)
            try container.encodeIfPresent(usagePeriod?.backendValue, forKey: .usagePeriod)
        case .accounts(let provider):
            try container.encodeIfPresent(provider, forKey: .provider)
        case .compatibility:
            try container.encode(1, forKey: .controlPlaneSchemaVersion)
            try container.encode(1, forKey: .stateSchemaVersion)
        case .settingsView, .aboutView:
            break
        case .updateSettings(let view):
            try container.encode(view, forKey: .view)
        case .supportReport(let includeDebugSummary, let recentDiagnostics):
            try container.encode(includeDebugSummary, forKey: .includeDebugSummary)
            try container.encode(recentDiagnostics, forKey: .recentDiagnostics)
        case .refresh(let provider, let kind, let targetKind, let localId, let usagePeriod):
            try container.encode(provider, forKey: .provider)
            try container.encode(kind, forKey: .kind)
            try container.encodeIfPresent(targetKind, forKey: .targetKind)
            try container.encodeIfPresent(localId, forKey: .localId)
            try container.encodeIfPresent(usagePeriod?.backendValue, forKey: .usagePeriod)
        case .switchTarget(let provider, let targetKind, let localId, let usagePeriod),
             .removeTarget(let provider, let targetKind, let localId, let usagePeriod):
            try container.encode(provider, forKey: .provider)
            try container.encode(targetKind, forKey: .targetKind)
            try container.encode(localId, forKey: .localId)
            try container.encodeIfPresent(usagePeriod?.backendValue, forKey: .usagePeriod)
        case .consumeResetCredit(let provider, let targetKind, let localId, let idempotencyKey, let usagePeriod):
            try container.encode(provider, forKey: .provider)
            try container.encode(targetKind, forKey: .targetKind)
            try container.encode(localId, forKey: .localId)
            try container.encode(idempotencyKey, forKey: .idempotencyKey)
            try container.encodeIfPresent(usagePeriod?.backendValue, forKey: .usagePeriod)
        case .login(let provider, let alias, let activate, let deviceAuth, let usagePeriod):
            try container.encode(provider, forKey: .provider)
            try container.encodeIfPresent(alias, forKey: .alias)
            try container.encode(activate, forKey: .activate)
            try container.encode(deviceAuth, forKey: .deviceAuth)
            try container.encodeIfPresent(usagePeriod?.backendValue, forKey: .usagePeriod)
        case .saveExistingLogin(let provider, let alias, let usagePeriod):
            try container.encode(provider, forKey: .provider)
            try container.encodeIfPresent(alias, forKey: .alias)
            try container.encodeIfPresent(usagePeriod?.backendValue, forKey: .usagePeriod)
        case .importProfile(let provider, let name, let content, let usagePeriod):
            try container.encode(provider, forKey: .provider)
            try container.encodeIfPresent(name, forKey: .name)
            try container.encode(content, forKey: .content)
            try container.encodeIfPresent(usagePeriod?.backendValue, forKey: .usagePeriod)
        case .cancelLogin:
            break
        }
    }

    private enum CodingKeys: String, CodingKey {
        case provider
        case usagePeriod = "usage_period"
        case controlPlaneSchemaVersion = "control_plane_schema_version"
        case stateSchemaVersion = "state_schema_version"
        case view
        case includeDebugSummary = "include_debug_summary"
        case recentDiagnostics = "recent_diagnostics"
        case kind
        case targetKind = "target_kind"
        case localId = "local_id"
        case idempotencyKey = "idempotency_key"
        case alias
        case activate
        case deviceAuth = "device_auth"
        case name
        case content
    }
}

public struct BackendEnvelope: Decodable, Sendable {
    public let schemaVersion: Int
    public let controlPlaneSchemaVersion: Int?
    public let stateSchemaVersion: Int?
    public let minimumBackendVersion: String?
    public let minimumFrontendVersion: String?
    public let ok: Bool
    public let dataStale: Bool?
    public let servedFromSnapshot: Bool?
    public let data: BackendData?
    public let error: BackendError?

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case controlPlaneSchemaVersion = "control_plane_schema_version"
        case stateSchemaVersion = "state_schema_version"
        case minimumBackendVersion = "minimum_backend_version"
        case minimumFrontendVersion = "minimum_frontend_version"
        case ok
        case dataStale = "data_stale"
        case servedFromSnapshot = "served_from_snapshot"
        case data
        case error
    }
}

public enum BackendData: Decodable, Sendable {
    case dashboard(DashboardReport)
    case accounts(AccountsReport)
    case operation(OperationData)
    case settings(SettingsView)
    case about(AboutView)
    case support(SupportReport)

    public var dashboard: DashboardReport? {
        if case .operation(let operation) = self {
            operation.dashboard
        } else if case .dashboard(let report) = self {
            report
        } else {
            nil
        }
    }

    public var operation: OperationResult? {
        if case .operation(let data) = self {
            data.operation
        } else {
            nil
        }
    }

    public var accounts: AccountsReport? {
        if case .accounts(let report) = self {
            report
        } else {
            nil
        }
    }

    public var settings: SettingsView? {
        if case .settings(let settings) = self {
            settings
        } else {
            nil
        }
    }

    public var about: AboutView? {
        if case .about(let about) = self {
            about
        } else {
            nil
        }
    }

    public var support: SupportReport? {
        if case .support(let report) = self {
            report
        } else {
            nil
        }
    }

    public init(from decoder: Decoder) throws {
        if let operation = try? OperationData(from: decoder) {
            self = .operation(operation)
            return
        }
        if let dashboard = try? DashboardReport(from: decoder) {
            self = .dashboard(dashboard)
            return
        }
        if let settings = try? SettingsView(from: decoder) {
            self = .settings(settings)
            return
        }
        if let about = try? AboutView(from: decoder) {
            self = .about(about)
            return
        }
        if let support = try? SupportReport(from: decoder) {
            self = .support(support)
            return
        }
        self = .accounts(try AccountsReport(from: decoder))
    }
}

public struct OperationData: Decodable, Sendable {
    public let operation: OperationResult
    public let dashboard: DashboardReport
    public let outcome: ResetCreditOutcome?
}

public enum ResetCreditOutcome: Decodable, Equatable, Sendable {
    case reset(windowsReset: UInt32)
    case nothingToReset
    case noCredit
    case alreadyRedeemed

    enum CodingKeys: String, CodingKey {
        case status
        case windowsReset = "windows_reset"
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(String.self, forKey: .status) {
        case "reset":
            self = .reset(windowsReset: try container.decodeIfPresent(UInt32.self, forKey: .windowsReset) ?? 0)
        case "nothing_to_reset":
            self = .nothingToReset
        case "no_credit":
            self = .noCredit
        case "already_redeemed":
            self = .alreadyRedeemed
        default:
            throw DecodingError.dataCorruptedError(forKey: .status, in: container, debugDescription: "Unknown reset credit outcome")
        }
    }
}

public struct BackendError: Decodable, Error, Sendable {
    public let code: String
    public let message: String
}

extension BackendError: LocalizedError {
    public var errorDescription: String? { message }
}

@_silgen_name("omx_menubar_call")
private func omx_menubar_call(_ requestJson: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?

@_silgen_name("omx_menubar_free")
private func omx_menubar_free(_ value: UnsafeMutablePointer<CChar>?)

private final class RustResponseString {
    private var raw: UnsafeMutablePointer<CChar>?

    init(_ raw: UnsafeMutablePointer<CChar>) {
        self.raw = raw
    }

    deinit {
        omx_menubar_free(raw)
    }

    func string() -> String {
        guard let raw else { return "" }
        return String(cString: raw)
    }
}

public struct RustBackendClient: BackendClient {
    public init() {}

    func call(_ request: BackendRequest) async throws -> BackendEnvelope {
        try await Task.detached(priority: .userInitiated) {
            let requestData = try JSONEncoder().encode(request)
            let requestJson = String(decoding: requestData, as: UTF8.self)
            guard let raw = requestJson.withCString({ omx_menubar_call($0) }) else {
                throw BackendError(code: "null_response", message: "backend returned null")
            }
            let response = RustResponseString(raw)
            let envelope = try JSONDecoder().decode(BackendEnvelope.self, from: Data(response.string().utf8))
            if let error = envelope.error, envelope.ok == false {
                throw error
            }
            return envelope
        }.value
    }
}
