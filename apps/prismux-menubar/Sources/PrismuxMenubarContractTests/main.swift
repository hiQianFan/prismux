import Foundation
import PrismuxMenubarCore

let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
let fixtureRoot = root.appendingPathComponent("crates/prismux-menubar-ffi/fixtures/menubar")
let decoder = JSONDecoder()

let accounts = try decode("accounts.response.json")
assert(accounts.ok)
assert(accounts.data?.accounts?.accounts.isEmpty == true)

let dashboard = try decode("dashboard.response.json")
assert(dashboard.ok)
assert(dashboard.data?.dashboard?.providerViews?.first?.statusTone == "neutral")

let refresh = try decode("refresh.response.json")
assert(refresh.ok)
assert(refresh.data?.dashboard?.accounts.accounts.isEmpty == true)
assert(refresh.data?.dashboard?.providerViews?.allSatisfy { $0.statusTone != nil } == true)

let failedSwitch = try decode("switch.response.json")
assert(!failedSwitch.ok)
assert(failedSwitch.error?.code == "application_error")

let additive = """
{
  "schema_version": 2,
  "ok": true,
  "data_stale": true,
  "served_from_snapshot": true,
  "data": {
    "generated_at_unix": 1,
    "unknown_optional": "ignored",
    "accounts": {
      "generated_at_unix": 1,
      "providers": ["codex"],
      "accounts": [],
      "profiles": [],
      "active_local_id": null,
      "active_target_key": null,
      "active_target_kind": null,
      "diagnostics": []
    },
    "active": null,
    "aggregate": {
      "quota_health": {
        "facts": {
          "account_count": 0,
          "reporting_count": 0,
          "avg_remaining_percent_x100": null,
          "min_remaining_percent_x100": null,
          "max_remaining_percent_x100": null,
          "soonest_reset_at_unix": null,
          "reset_credit_total": 0
        },
        "healthy_count": 0,
        "low_count": 0,
        "exhausted_count": 0,
        "worst_target": null,
        "best_alternative": null,
        "status": "unavailable",
        "status_tone": "danger"
      },
      "provider_aggregates": [],
      "diagnostics": []
    }
  }
}
"""
let additiveEnvelope = try decoder.decode(BackendEnvelope.self, from: Data(additive.utf8))
assert(additiveEnvelope.dataStale == true)
assert(additiveEnvelope.servedFromSnapshot == true)

let resetCreditsAccounts = """
{
  "schema_version": 2,
  "ok": true,
  "data": {
    "generated_at_unix": 1,
    "providers": ["codex"],
    "accounts": [
      {
        "provider": "codex",
        "account_key": "codex/account/local-1",
        "target_kind": "account",
        "display_number": 1,
        "local_id": "local-1",
        "display_label": "work",
        "secondary_label": "work@example.com",
        "alias": "work",
        "account_label": "work@example.com",
        "plan": "Pro",
        "auth_type": "chatgpt",
        "active": true,
        "quota": {
          "summary": "0%",
          "refreshed_at_unix": 1,
          "primary_window": null,
          "windows": [],
          "reset_credits": { "available_count": 2 }
        },
        "status": "limited",
        "actions": {
          "can_activate": false,
          "can_remove": true,
          "primary_label": "Use this account",
          "disabled_reason": "already_active"
        },
        "diagnostic": null
      },
      {
        "provider": "codex",
        "account_key": "codex/account/local-2",
        "target_kind": "account",
        "display_number": 2,
        "local_id": "local-2",
        "display_label": "other",
        "secondary_label": "other@example.com",
        "alias": null,
        "account_label": "other@example.com",
        "plan": "Pro",
        "auth_type": "chatgpt",
        "active": false,
        "quota": {
          "summary": "80%",
          "refreshed_at_unix": 1,
          "primary_window": null,
          "windows": []
        },
        "status": "healthy",
        "actions": {
          "can_activate": true,
          "can_remove": true,
          "primary_label": "Use this account",
          "disabled_reason": null
        },
        "diagnostic": null
      }
    ],
    "profiles": [],
    "active_local_id": "local-1",
    "active_target_key": "codex/account/local-1",
    "active_target_kind": "account",
    "diagnostics": []
  }
}
"""
let resetCreditsEnvelope = try decoder.decode(BackendEnvelope.self, from: Data(resetCreditsAccounts.utf8))
let resetCreditAccounts = resetCreditsEnvelope.data?.accounts?.accounts ?? []
assert(resetCreditAccounts.first?.quota?.resetCredits?.availableCount == 2)
assert(resetCreditAccounts.first?.quota?.resetCredits?.credits.isEmpty == true)
assert(resetCreditAccounts.last?.quota?.resetCredits == nil)

let resetCreditsWithExpiry = """
{
  "summary": "0%",
  "refreshed_at_unix": 1,
  "primary_window": null,
  "windows": [],
  "reset_credits": {
    "available_count": 2,
    "credits": [
      {
        "status": "available",
        "reset_type": "codex_rate_limits",
        "granted_at_unix": 1781742467,
        "expires_at_unix": 1784334467
      },
      {
        "status": "available",
        "reset_type": "codex_rate_limits",
        "granted_at_unix": 1782528081,
        "expires_at_unix": 1785120081
      }
    ]
  }
}
"""
let resetCreditsQuota = try decoder.decode(Quota.self, from: Data(resetCreditsWithExpiry.utf8))
assert(resetCreditsQuota.resetCredits?.availableCount == 2)
assert(resetCreditsQuota.resetCredits?.credits.count == 2)
assert(resetCreditsQuota.resetCredits?.credits.first?.expiresAtUnix == 1784334467)

let twoExpiryHover = resetCreditHoverText(
    count: 2,
    expiryTimes: [1785120081, 1784334467]
)
assert(twoExpiryHover.contains("2 resets available"))
assert(twoExpiryHover.components(separatedBy: "2026-").count - 1 == 2)
assert(!twoExpiryHover.contains("Used automatically"))

let threeExpiryHover = resetCreditHoverText(
    count: 3,
    expiryTimes: [1785120081, 1784334467, 1785200000]
)
assert(threeExpiryHover.components(separatedBy: "2026-").count - 1 == 3)

let duplicateExpiryHover = resetCreditHoverText(
    count: 3,
    expiryTimes: [1785120081, 1784334467, 1784334467]
)
assert(duplicateExpiryHover.components(separatedBy: "2026-").count - 1 == 3)
assert(!duplicateExpiryHover.contains("x2"))

let countOnlyHover = resetCreditHoverText(count: 2, expiryTimes: [])
assert(countOnlyHover.contains("Expiry unavailable"))

func quotaWindows(_ json: String) throws -> [QuotaWindow] {
    try decoder.decode([QuotaWindow].self, from: Data(json.utf8))
}

let standardQuotaWindows = try quotaWindows("""
[
  {"id":"primary","label":"5h","window_seconds":18000},
  {"id":"secondary","label":"weekly","window_seconds":604800}
]
""")
let standardSelection = selectCodexQuotaWindows(standardQuotaWindows)
assert(standardSelection.slots.map(\.label) == ["5h", "7d"])
assert(standardSelection.slots.map(\.window.id) == ["primary", "secondary"])

let weeklyOnlySelection = selectCodexQuotaWindows(try quotaWindows("""
[{"id":"primary","label":"weekly","window_seconds":604800}]
"""))
assert(weeklyOnlySelection.slots.map(\.label) == ["7d"])
assert(weeklyOnlySelection.slots.map(\.window.id) == ["primary"])

let shortOnlySelection = selectCodexQuotaWindows(try quotaWindows("""
[{"id":"primary","label":"session","window_seconds":18000}]
"""))
assert(shortOnlySelection.slots.map(\.label) == ["5h"])
assert(shortOnlySelection.slots.map(\.window.id) == ["primary"])

let monthlyAndUnsupportedSelection = selectCodexQuotaWindows(try quotaWindows("""
[
  {"id":"monthly","label":"30d","window_seconds":2592000},
  {"id":"daily","label":"1d","window_seconds":86400},
  {"id":"unknown","label":"rolling"}
]
"""))
assert(monthlyAndUnsupportedSelection.slots.map(\.label) == ["30d"])
assert(monthlyAndUnsupportedSelection.slots.map(\.window.id) == ["monthly"])

let mixedSelection = selectCodexQuotaWindows(try quotaWindows("""
[
  {"id":"monthly","label":"30d","window_seconds":2592000},
  {"id":"short","label":"session","window_seconds":18000},
  {"id":"weekly","label":"7d","window_seconds":604800}
]
"""))
assert(mixedSelection.slots.map(\.label) == ["5h", "7d"])
assert(mixedSelection.slots.map(\.window.id) == ["short", "weekly"])

let shiftedSelection = selectCodexQuotaWindows(try quotaWindows("""
[
  {"id":"weekly","label":"7d","window_seconds":604800},
  {"id":"monthly","label":"30d","window_seconds":2592000}
]
"""))
assert(shiftedSelection.slots.map(\.label) == ["7d", "30d"])
assert(shiftedSelection.slots.map(\.window.id) == ["weekly", "monthly"])

let gappedSelection = selectCodexQuotaWindows(try quotaWindows("""
[
  {"id":"short","label":"5h","window_seconds":18000},
  {"id":"monthly","label":"30d","window_seconds":2592000}
]
"""))
assert(gappedSelection.slots.map(\.label) == ["5h"])
assert(gappedSelection.slots.map(\.window.id) == ["short"])

let structuredDurationWins = selectCodexQuotaWindows(try quotaWindows("""
[
  {"id":"misleading-week","label":"weekly","window_seconds":18000},
  {"id":"misleading-short","label":"5h session","window_seconds":604800}
]
"""))
assert(structuredDurationWins.slots.map(\.label) == ["5h", "7d"])
assert(structuredDurationWins.slots.map(\.window.id) == ["misleading-week", "misleading-short"])

let legacyTextSelection = selectCodexQuotaWindows(try quotaWindows("""
[
  {"id":"legacy-session","label":"short"},
  {"id":"legacy-week","label":"weekly"}
]
"""))
assert(legacyTextSelection.slots.map(\.label) == ["5h", "7d"])
assert(legacyTextSelection.slots.map(\.window.id) == ["legacy-session", "legacy-week"])

let shiftedLegacyTextSelection = selectCodexQuotaWindows(try quotaWindows("""
[
  {"id":"legacy-week","label":"weekly"},
  {"id":"legacy-month","label":"monthly"}
]
"""))
assert(shiftedLegacyTextSelection.slots.map(\.label) == ["7d", "30d"])
assert(shiftedLegacyTextSelection.slots.map(\.window.id) == ["legacy-week", "legacy-month"])

let structuredMonthlyWins = selectCodexQuotaWindows(try quotaWindows("""
[
  {"id":"legacy-month","label":"30d"},
  {"id":"structured-month","label":"unknown","window_seconds":2592000}
]
"""))
assert(structuredMonthlyWins.slots.map(\.label) == ["30d"])
assert(structuredMonthlyWins.slots.map(\.window.id) == ["structured-month"])

let ambiguousLegacyWindow = selectCodexQuotaWindows(try quotaWindows("""
[{"id":"short-week","label":"5h weekly"}]
"""))
assert(ambiguousLegacyWindow.slots.map(\.label) == ["5h"])
assert(ambiguousLegacyWindow.slots.map(\.window.id) == ["short-week"])

let substringSelection = selectCodexQuotaWindows(try quotaWindows("""
[
  {"id":"fifteen-hours","label":"15h"},
  {"id":"weekend-promo","label":"rolling"},
  {"id":"shortage","label":"unknown"}
]
"""))
assert(substringSelection.slots.isEmpty)

let structuredSelectionWinsGlobally = selectCodexQuotaWindows(try quotaWindows("""
[
  {"id":"legacy-short","label":"5h"},
  {"id":"structured-short","label":"primary","window_seconds":18000},
  {"id":"legacy-week","label":"weekly"},
  {"id":"structured-week","label":"secondary","window_seconds":604800}
]
"""))
assert(structuredSelectionWinsGlobally.slots.map(\.label) == ["5h", "7d"])
assert(structuredSelectionWinsGlobally.slots.map(\.window.id) == ["structured-short", "structured-week"])

func decode(_ name: String) throws -> BackendEnvelope {
    let data = try Data(contentsOf: fixtureRoot.appendingPathComponent(name))
    return try decoder.decode(BackendEnvelope.self, from: data)
}

print("PrismuxMenubarContractTests passed")
