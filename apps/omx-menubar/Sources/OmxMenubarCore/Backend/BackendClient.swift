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
    case dashboard(provider: String?)
    case accounts(provider: String?)
    case refresh(provider: String, kind: String)
    case switchTarget(provider: String, targetKind: String, localId: String)
    case removeTarget(provider: String, targetKind: String, localId: String)

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .dashboard(let provider), .accounts(let provider):
            try container.encodeIfPresent(provider, forKey: .provider)
        case .refresh(let provider, let kind):
            try container.encode(provider, forKey: .provider)
            try container.encode(kind, forKey: .kind)
        case .switchTarget(let provider, let targetKind, let localId),
             .removeTarget(let provider, let targetKind, let localId):
            try container.encode(provider, forKey: .provider)
            try container.encode(targetKind, forKey: .targetKind)
            try container.encode(localId, forKey: .localId)
        }
    }

    private enum CodingKeys: String, CodingKey {
        case provider
        case kind
        case targetKind = "target_kind"
        case localId = "local_id"
    }
}

public struct BackendEnvelope: Decodable, Sendable {
    public let schemaVersion: Int
    public let ok: Bool
    public let data: BackendData?
    public let error: BackendError?

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case ok
        case data
        case error
    }
}

public enum BackendData: Decodable, Sendable {
    case dashboard(DashboardReport)
    case accounts(AccountsReport)
    case operation(OperationData)

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

    public init(from decoder: Decoder) throws {
        if let operation = try? OperationData(from: decoder) {
            self = .operation(operation)
            return
        }
        if let dashboard = try? DashboardReport(from: decoder) {
            self = .dashboard(dashboard)
            return
        }
        self = .accounts(try AccountsReport(from: decoder))
    }
}

public struct OperationData: Decodable, Sendable {
    public let operation: OperationResult
    public let dashboard: DashboardReport
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
