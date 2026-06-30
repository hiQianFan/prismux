import Foundation

public struct DashboardReport: Decodable, Sendable {
    public let controlPlaneSchemaVersion: UInt32?
    public let stateSchemaVersion: UInt32?
    public let generatedAtUnix: UInt64
    public let accounts: AccountsReport
    public let active: TargetAccount?
    public let providerViews: [ProviderView]?
    public let aggregate: DashboardAggregateView
    public let usage: UsageSummary
    public let providerUsage: [ProviderUsageSummary]

    enum CodingKeys: String, CodingKey {
        case controlPlaneSchemaVersion = "control_plane_schema_version"
        case stateSchemaVersion = "state_schema_version"
        case generatedAtUnix = "generated_at_unix"
        case accounts
        case active
        case providerViews = "provider_views"
        case aggregate
        case usage
        case providerUsage = "provider_usage"
    }
}

public struct ProviderView: Decodable, Sendable {
    public let provider: String
    public let displayLabel: String
    public let status: String
    public let statusText: String
    public let statusTone: String?
    public let targetCount: Int
    public let aggregate: ProviderAggregateView?
    public let diagnostics: [Diagnostic]

    enum CodingKeys: String, CodingKey {
        case provider
        case displayLabel = "display_label"
        case status
        case statusText = "status_text"
        case statusTone = "status_tone"
        case targetCount = "target_count"
        case aggregate
        case diagnostics
    }
}

public struct DashboardAggregateView: Decodable, Sendable {
    public let quotaHealth: QuotaHealthRollup
    public let providerAggregates: [ProviderAggregateView]
    public let usageHeadline: UsageHeadline
    public let diagnostics: [Diagnostic]

    enum CodingKeys: String, CodingKey {
        case quotaHealth = "quota_health"
        case providerAggregates = "provider_aggregates"
        case usageHeadline = "usage_headline"
        case diagnostics
    }
}

public struct ProviderAggregateView: Decodable, Sendable {
    public let providerId: String
    public let providerDisplayLabel: String
    public let accountCount: UInt32
    public let profileCount: UInt32
    public let targetCount: UInt32
    public let activeTarget: ActiveTarget?
    public let quotaHealth: QuotaHealthRollup
    /// This provider's token/cost headline for the selected period.
    public let usageHeadline: UsageHeadline?
    public let status: String
    public let statusTone: String
    public let statusText: String
    public let diagnostics: [Diagnostic]

    enum CodingKeys: String, CodingKey {
        case providerId = "provider_id"
        case providerDisplayLabel = "provider_display_label"
        case accountCount = "account_count"
        case profileCount = "profile_count"
        case targetCount = "target_count"
        case activeTarget = "active_target"
        case quotaHealth = "quota_health"
        case usageHeadline = "usage_headline"
        case status
        case statusTone = "status_tone"
        case statusText = "status_text"
        case diagnostics
    }
}

public struct QuotaHealthRollup: Decodable, Sendable {
    public let facts: QuotaFactsRollup
    public let healthyCount: UInt32
    public let lowCount: UInt32
    public let exhaustedCount: UInt32
    public let worstTarget: ActiveTarget?
    public let bestAlternative: TargetRecommendation?
    public let windowAverages: WindowAverages?
    public let status: String
    public let statusTone: String

    enum CodingKeys: String, CodingKey {
        case facts
        case healthyCount = "healthy_count"
        case lowCount = "low_count"
        case exhaustedCount = "exhausted_count"
        case worstTarget = "worst_target"
        case bestAlternative = "best_alternative"
        case windowAverages = "window_averages"
        case status
        case statusTone = "status_tone"
    }
}

/// Per-window-class average remaining (x100). nil = that class had no reporting
/// account. The Overview renders 5h / 7d bars from these, identical to the
/// account card's QuotaLine.
public struct WindowAverages: Decodable, Sendable {
    public let shortRemainingPercentX100: UInt32?
    public let weeklyRemainingPercentX100: UInt32?

    enum CodingKeys: String, CodingKey {
        case shortRemainingPercentX100 = "short_remaining_percent_x100"
        case weeklyRemainingPercentX100 = "weekly_remaining_percent_x100"
    }
}

public struct QuotaFactsRollup: Decodable, Sendable {
    public let accountCount: UInt32
    public let reportingCount: UInt32
    public let avgRemainingPercentX100: UInt32?
    public let minRemainingPercentX100: UInt32?
    public let maxRemainingPercentX100: UInt32?
    public let soonestResetAtUnix: Int64?
    public let resetCreditTotal: UInt32

    enum CodingKeys: String, CodingKey {
        case accountCount = "account_count"
        case reportingCount = "reporting_count"
        case avgRemainingPercentX100 = "avg_remaining_percent_x100"
        case minRemainingPercentX100 = "min_remaining_percent_x100"
        case maxRemainingPercentX100 = "max_remaining_percent_x100"
        case soonestResetAtUnix = "soonest_reset_at_unix"
        case resetCreditTotal = "reset_credit_total"
    }
}

public struct TargetRecommendation: Decodable, Sendable {
    public let target: ActiveTarget
    public let reason: String
    public let action: String
}

public struct UsageHeadline: Decodable, Sendable {
    public let totalTokens: UInt64
    public let inputTokens: UInt64?
    public let outputTokens: UInt64?
    public let estimatedCostUsd: String?
    public let costStatus: String
    public let topClient: String?
    public let topModel: String?
    public let breakdown: [UsageModelBreakdown]

    enum CodingKeys: String, CodingKey {
        case totalTokens = "total_tokens"
        case inputTokens = "input_tokens"
        case outputTokens = "output_tokens"
        case estimatedCostUsd = "estimated_cost_usd"
        case costStatus = "cost_status"
        case topClient = "top_client"
        case topModel = "top_model"
        case breakdown
    }
}

public struct AccountsReport: Decodable, Sendable {
    public let controlPlaneSchemaVersion: UInt32?
    public let stateSchemaVersion: UInt32?
    public let providers: [String]
    public let accounts: [TargetAccount]
    public let profiles: [TargetProfile]
    public let activeLocalId: String?
    public let activeTargetKey: String?
    public let activeTargetKind: String?
    public let systemActiveTarget: ActiveTarget?
    public let selectedUiTarget: ActiveTarget?
    public let refreshScopeTarget: ActiveTarget?
    public let observedTarget: ActiveTarget?
    public let diagnostics: [Diagnostic]

    enum CodingKeys: String, CodingKey {
        case controlPlaneSchemaVersion = "control_plane_schema_version"
        case stateSchemaVersion = "state_schema_version"
        case providers
        case accounts
        case profiles
        case activeLocalId = "active_local_id"
        case activeTargetKey = "active_target_key"
        case activeTargetKind = "active_target_kind"
        case systemActiveTarget = "system_active_target"
        case selectedUiTarget = "selected_ui_target"
        case refreshScopeTarget = "refresh_scope_target"
        case observedTarget = "observed_target"
        case diagnostics
    }
}

public struct TargetAccount: Decodable, Identifiable, Sendable {
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
    public let actions: TargetActions?
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
        case actions
        case diagnostic
    }
}

public struct TargetProfile: Decodable, Identifiable, Sendable {
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
    public let actions: TargetActions?
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
        case actions
        case diagnostic
    }
}

public struct TargetActions: Decodable, Sendable {
    public let canActivate: Bool
    public let canRemove: Bool
    public let primaryLabel: String
    public let disabledReason: String?

    enum CodingKeys: String, CodingKey {
        case canActivate = "can_activate"
        case canRemove = "can_remove"
        case primaryLabel = "primary_label"
        case disabledReason = "disabled_reason"
    }
}

public struct Quota: Decodable, Sendable {
    public let summary: String
    public let refreshedAtUnix: Int64?
    public let primaryWindow: QuotaWindow?
    public let windows: [QuotaWindow]
    public let resetCredits: ResetCredits?

    enum CodingKeys: String, CodingKey {
        case summary
        case refreshedAtUnix = "refreshed_at_unix"
        case primaryWindow = "primary_window"
        case windows
        case resetCredits = "reset_credits"
    }
}

public struct ResetCredits: Decodable, Sendable {
    public let availableCount: UInt32
    public let credits: [ResetCredit]

    enum CodingKeys: String, CodingKey {
        case availableCount = "available_count"
        case credits
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        availableCount = try container.decode(UInt32.self, forKey: .availableCount)
        credits = try container.decodeIfPresent([ResetCredit].self, forKey: .credits) ?? []
    }
}

public struct ResetCredit: Decodable, Sendable {
    public let status: String?
    public let resetType: String?
    public let grantedAtUnix: Int64?
    public let expiresAtUnix: Int64?

    enum CodingKeys: String, CodingKey {
        case status
        case resetType = "reset_type"
        case grantedAtUnix = "granted_at_unix"
        case expiresAtUnix = "expires_at_unix"
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
    public let hourlyBuckets: [HourlyBucket]?
    public let series: [UsageChartSeries]?
    public let coverage: Coverage

    enum CodingKeys: String, CodingKey {
        case totalTokens = "total_tokens"
        case topClient = "top_client"
        case topModel = "top_model"
        case modelBreakdown = "model_breakdown"
        case hourlyBuckets = "hourly_buckets"
        case series
        case coverage
    }
}

/// One local-hour bucket of token usage. The hour is the canonical unit: the
/// UI renders today as hourly bars and rolls hours up into days (the
/// `YYYY-MM-DD` prefix of `localHour`) for the 7d/30d views.
public struct HourlyBucket: Decodable, Sendable {
    public let localHour: String
    public let totalTokens: UInt64
    public let estimatedCostUsd: String?
    public let costStatus: String?

    enum CodingKeys: String, CodingKey {
        case localHour = "local_hour"
        case totalTokens = "total_tokens"
        case estimatedCostUsd = "estimated_cost_usd"
        case costStatus = "cost_status"
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

public struct UsageChartSeries: Decodable, Sendable {
    public let kind: String
    public let key: String
    public let label: String
    public let hourlyBuckets: [HourlyBucket]

    enum CodingKeys: String, CodingKey {
        case kind
        case key
        case label
        case hourlyBuckets = "hourly_buckets"
    }
}

public struct ProviderUsageSummary: Decodable, Sendable {
    public let provider: String
    public let usage: UsageSummary
}

public struct Coverage: Decodable, Sendable {
    public let status: String
    public let tone: String?
}

public struct Diagnostic: Decodable, Sendable {
    public let code: String
    public let message: String
    public let providerId: String?
    public let targetId: String?
    public let scope: String?
    public let recoveryAction: String?

    enum CodingKeys: String, CodingKey {
        case code
        case message
        case providerId = "provider_id"
        case targetId = "target_id"
        case scope
        case recoveryAction = "recovery_action"
    }
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

public struct SettingsView: Codable, Sendable {
    public var schemaVersion: UInt32
    public var general: GeneralSettings
    public var providers: [ProviderSettings]
    public var privacy: PrivacySettings

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case general
        case providers
        case privacy
    }
}

public struct GeneralSettings: Codable, Sendable {
    public var refreshCadenceSeconds: UInt64

    enum CodingKeys: String, CodingKey {
        case refreshCadenceSeconds = "refresh_cadence_seconds"
    }
}

public struct PrivacySettings: Codable, Sendable {
    public var hidePersonalIdentifiers: Bool

    enum CodingKeys: String, CodingKey {
        case hidePersonalIdentifiers = "hide_personal_identifiers"
    }
}

public struct ProviderSettings: Codable, Identifiable, Sendable {
    public var id: String { provider }

    public var provider: String
    public var displayLabel: String
    public var enabled: Bool
    public var status: ProviderSettingsStatus
    public var sourcePreference: SourcePreference
    public var sourceOptions: [SettingsPickerOption]
    public var diagnostics: [SettingsDiagnostic]

    enum CodingKeys: String, CodingKey {
        case provider
        case displayLabel = "display_label"
        case enabled
        case status
        case sourcePreference = "source_preference"
        case sourceOptions = "source_options"
        case diagnostics
    }
}

public struct ProviderSettingsStatus: Codable, Sendable {
    public var status: String
    public var statusText: String
    public var statusTone: String

    enum CodingKeys: String, CodingKey {
        case status
        case statusText = "status_text"
        case statusTone = "status_tone"
    }
}

public struct SettingsPickerOption: Codable, Identifiable, Sendable {
    public var id: SourcePreference { value }

    public var value: SourcePreference
    public var label: String
    public var disabledReason: String?

    enum CodingKeys: String, CodingKey {
        case value
        case label
        case disabledReason = "disabled_reason"
    }
}

public struct SettingsDiagnostic: Codable, Sendable {
    public var code: String
    public var message: String
    public var recoveryAction: String?

    enum CodingKeys: String, CodingKey {
        case code
        case message
        case recoveryAction = "recovery_action"
    }
}

public enum SourcePreference: String, Codable, Sendable {
    case auto
    case localOnly = "local_only"
    case remoteOnly = "remote_only"
}

public struct AboutView: Decodable, Sendable {
    public let schemaVersion: UInt32
    public let appVersion: String
    public let controlPlaneSchemaVersion: UInt32
    public let stateSchemaVersion: UInt32
    public let settingsSchemaVersion: UInt32
    public let runtime: AboutRuntime
    public let stateRoot: AboutPath
    public let settingsPath: AboutPath
    public let links: [AboutLink]
    public let authorLinks: [AboutLink]

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case appVersion = "app_version"
        case controlPlaneSchemaVersion = "control_plane_schema_version"
        case stateSchemaVersion = "state_schema_version"
        case settingsSchemaVersion = "settings_schema_version"
        case runtime
        case stateRoot = "state_root"
        case settingsPath = "settings_path"
        case links
        case authorLinks = "author_links"
    }
}

public struct AboutRuntime: Decodable, Sendable {
    public let mode: String
    public let statusText: String

    enum CodingKeys: String, CodingKey {
        case mode
        case statusText = "status_text"
    }
}

public struct AboutPath: Decodable, Sendable {
    public let display: String
    public let revealPath: String?

    enum CodingKeys: String, CodingKey {
        case display
        case revealPath = "reveal_path"
    }
}

public struct AboutLink: Decodable, Identifiable, Sendable {
    public var id: String { url }

    public let label: String
    public let url: String
}

public struct SupportReport: Decodable, Sendable {
    public let schemaVersion: UInt32
    public let appVersion: String
    public let controlPlaneSchemaVersion: UInt32
    public let stateSchemaVersion: UInt32
    public let settingsSchemaVersion: UInt32
    public let redactionStatus: String
    public let diagnostics: [SupportDiagnostic]

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

public struct SupportDiagnostic: Decodable, Sendable {
    public let code: String
    public let severity: String
    public let userMessage: String
    public let recoveryAction: String?
    public let source: String
    public let redactionStatus: String

    enum CodingKeys: String, CodingKey {
        case code
        case severity
        case userMessage = "user_message"
        case recoveryAction = "recovery_action"
        case source
        case redactionStatus = "redaction_status"
    }
}
