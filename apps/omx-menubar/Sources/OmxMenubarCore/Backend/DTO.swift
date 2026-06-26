import Foundation

public struct DashboardReport: Decodable, Sendable {
    public let generatedAtUnix: UInt64
    public let accounts: AccountsReport
    public let active: MenubarAccount?
    public let usage: UsageSummary
    public let providerUsage: [ProviderUsageSummary]

    enum CodingKeys: String, CodingKey {
        case generatedAtUnix = "generated_at_unix"
        case accounts
        case active
        case usage
        case providerUsage = "provider_usage"
    }
}

public struct AccountsReport: Decodable, Sendable {
    public let providers: [String]
    public let accounts: [MenubarAccount]
    public let profiles: [MenubarProfile]
    public let activeLocalId: String?
    public let activeTargetKey: String?
    public let activeTargetKind: String?
    public let diagnostics: [Diagnostic]

    enum CodingKeys: String, CodingKey {
        case providers
        case accounts
        case profiles
        case activeLocalId = "active_local_id"
        case activeTargetKey = "active_target_key"
        case activeTargetKind = "active_target_kind"
        case diagnostics
    }
}

public struct MenubarAccount: Decodable, Identifiable, Sendable {
    public var id: String { accountKey }
    public var shortLabel: String { displayLabel }

    public let provider: String
    public let accountKey: String
    public let targetKind: String
    public let displayNumber: Int
    public let localId: String
    public let displayLabel: String
    public let secondaryLabel: String
    public let alias: String?
    public let accountLabel: String?
    public let plan: String?
    public let authType: String?
    public let active: Bool
    public let quota: Quota?
    public let status: String
    public let diagnostic: Diagnostic?

    enum CodingKeys: String, CodingKey {
        case provider
        case accountKey = "account_key"
        case targetKind = "target_kind"
        case displayNumber = "display_number"
        case localId = "local_id"
        case displayLabel = "display_label"
        case secondaryLabel = "secondary_label"
        case alias
        case accountLabel = "account_label"
        case plan
        case authType = "auth_type"
        case active
        case quota
        case status
        case diagnostic
    }
}

public struct MenubarProfile: Decodable, Identifiable, Sendable {
    public var id: String { accountKey }

    public let provider: String
    public let accountKey: String
    public let targetKind: String
    public let displayNumber: Int
    public let localId: String
    public let displayLabel: String
    public let secondaryLabel: String
    public let name: String
    public let active: Bool
    public let providerId: String?
    public let baseUrl: String?
    public let model: String?
    public let authType: String?
    public let status: String
    public let diagnostic: Diagnostic?

    enum CodingKeys: String, CodingKey {
        case provider
        case accountKey = "account_key"
        case targetKind = "target_kind"
        case displayNumber = "display_number"
        case localId = "local_id"
        case displayLabel = "display_label"
        case secondaryLabel = "secondary_label"
        case name
        case active
        case providerId = "provider_id"
        case baseUrl = "base_url"
        case model
        case authType = "auth_type"
        case status
        case diagnostic
    }
}

public struct Quota: Decodable, Sendable {
    public let summary: String
    public let refreshedAtUnix: Int64?
    public let primaryWindow: QuotaWindow?
    public let windows: [QuotaWindow]

    enum CodingKeys: String, CodingKey {
        case summary
        case refreshedAtUnix = "refreshed_at_unix"
        case primaryWindow = "primary_window"
        case windows
    }
}

public struct QuotaWindow: Decodable, Sendable {
    public let id: String
    public let label: String
    public let windowSeconds: UInt64?
    public let usedPercentX100: UInt32?
    public let remainingPercentX100: UInt32?
    public let resetAtUnix: Int64?
    public let exhausted: Bool?

    enum CodingKeys: String, CodingKey {
        case id
        case label
        case windowSeconds = "window_seconds"
        case usedPercentX100 = "used_percent_x100"
        case remainingPercentX100 = "remaining_percent_x100"
        case resetAtUnix = "reset_at_unix"
        case exhausted
    }
}

public struct UsageSummary: Decodable, Sendable {
    public let totalTokens: UInt64
    public let topClient: String?
    public let topModel: String?
    public let modelBreakdown: [UsageModelBreakdown]
    public let coverage: Coverage

    enum CodingKeys: String, CodingKey {
        case totalTokens = "total_tokens"
        case topClient = "top_client"
        case topModel = "top_model"
        case modelBreakdown = "model_breakdown"
        case coverage
    }
}

public struct UsageModelBreakdown: Decodable, Sendable {
    public let model: String
    public let totalTokens: UInt64

    enum CodingKeys: String, CodingKey {
        case model
        case totalTokens = "total_tokens"
    }
}

public struct ProviderUsageSummary: Decodable, Sendable {
    public let provider: String
    public let usage: UsageSummary
}

public struct Coverage: Decodable, Sendable {
    public let status: String
}

public struct Diagnostic: Decodable, Sendable {
    public let code: String
    public let message: String
}

public struct OperationResult: Decodable, Sendable {
    public let status: String
    public let changed: Bool
    public let activeBefore: ActiveTarget?
    public let activeAfter: ActiveTarget?
    public let message: String
    public let diagnostics: [Diagnostic]

    enum CodingKeys: String, CodingKey {
        case status
        case changed
        case activeBefore = "active_before"
        case activeAfter = "active_after"
        case message
        case diagnostics
    }
}

public struct ActiveTarget: Decodable, Sendable {
    public let provider: String
    public let targetKind: String
    public let localId: String
    public let accountKey: String
    public let displayLabel: String

    enum CodingKeys: String, CodingKey {
        case provider
        case targetKind = "target_kind"
        case localId = "local_id"
        case accountKey = "account_key"
        case displayLabel = "display_label"
    }
}
