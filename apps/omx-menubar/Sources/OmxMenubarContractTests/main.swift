import Foundation
import OmxMenubarCore

let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
let fixtureRoot = root.appendingPathComponent("crates/omx-menubar-ffi/fixtures/menubar")
let decoder = JSONDecoder()

let accounts = try decode("accounts.response.json")
assert(accounts.ok)
assert(accounts.data?.accounts?.accounts.isEmpty == true)

let dashboard = try decode("dashboard.response.json")
assert(dashboard.ok)
assert(dashboard.data?.dashboard?.usage.coverage.status == "empty")
assert(dashboard.data?.dashboard?.usage.coverage.tone == "warning")
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
  "schema_version": 1,
  "ok": true,
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
    "usage": {
      "period": "Today",
      "total_tokens": 0,
      "top_client": null,
      "top_model": null,
      "model_breakdown": [],
      "hourly_buckets": [
        {"local_hour": "2026-06-27T09", "total_tokens": 120},
        {"local_hour": "2026-06-27T10", "total_tokens": 80},
        {"local_hour": "2026-06-26T09", "total_tokens": 200}
      ],
      "cost_status": "Missing",
      "estimated_cost_usd": null,
      "freshness": {"generated_at_unix": 1, "stale": true},
      "coverage": {
        "status": "empty",
        "requested_clients": [],
        "available_clients": [],
        "missing_clients": []
      }
    },
    "provider_usage": [
      {
        "provider": "codex",
        "usage": {
          "period": "Today",
          "total_tokens": 0,
          "top_client": null,
          "top_model": null,
          "model_breakdown": [],
          "cost_status": "Missing",
          "estimated_cost_usd": null,
          "freshness": {"generated_at_unix": 1, "stale": true},
          "coverage": {
            "status": "empty",
            "requested_clients": [],
            "available_clients": [],
            "missing_clients": []
          }
        }
      }
    ]
  }
}
"""
let additiveEnvelope = try decoder.decode(BackendEnvelope.self, from: Data(additive.utf8))
assert(additiveEnvelope.data?.dashboard?.usage.coverage.status == "empty")

let resetCreditsAccounts = """
{
  "schema_version": 1,
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
assert(resetCreditAccounts.last?.quota?.resetCredits == nil)

// hourly_buckets decode + rollup
let usage = additiveEnvelope.data?.dashboard?.usage
assert(usage?.hourlyBuckets?.count == 3)

var anchorComponents = DateComponents()
anchorComponents.year = 2026
anchorComponents.month = 6
anchorComponents.day = 27
anchorComponents.hour = 11
var utcCalendar = Calendar(identifier: .gregorian)
utcCalendar.timeZone = TimeZone(identifier: "UTC")!
let anchor = utcCalendar.date(from: anchorComponents)!
let buckets = usage?.hourlyBuckets ?? []

// Today: 24 hourly bars, only the two 2026-06-27 hours have tokens.
let todayBars = UsageSeries.bars(from: buckets, period: .today, now: anchor, calendar: utcCalendar)
assert(todayBars.count == 24)
assert(UsageSeries.total(todayBars) == 200) // 120 + 80, the 06-26 bucket excluded
assert(todayBars[9].tokens == 120 && todayBars[10].tokens == 80)
assert(todayBars[11].isCurrent)

// 7d: one bar per day, current day last; the 06-26 bucket now counts.
let weekBars = UsageSeries.bars(from: buckets, period: .sevenDays, now: anchor, calendar: utcCalendar)
assert(weekBars.count == 7)
assert(UsageSeries.total(weekBars) == 400) // 120 + 80 + 200
assert(weekBars.last?.isCurrent == true && weekBars.last?.tokens == 200)
assert(weekBars[weekBars.count - 2].tokens == 200) // yesterday 06-26

// Stacked: same buckets split across two providers; per-bar tokens must equal
// the sum of segments, and segments must carry the right provider tokens.
let stacked = UsageSeries.stackedBars(
    from: [(provider: "codex", buckets: buckets), (provider: "claude", buckets: [])],
    period: .today,
    now: anchor,
    calendar: utcCalendar
)
assert(stacked.count == 24)
assert(stacked[9].tokens == 120)
assert(stacked[9].segments.count == 2)
assert(stacked[9].segments.first { $0.provider == "codex" }?.tokens == 120)
assert(stacked[9].segments.first { $0.provider == "claude" }?.tokens == 0)
// Bar total is always the sum of its segments.
assert(stacked.allSatisfy { $0.tokens == $0.segments.reduce(UInt64(0)) { $0 + $1.tokens } })

func decode(_ name: String) throws -> BackendEnvelope {
    let data = try Data(contentsOf: fixtureRoot.appendingPathComponent(name))
    return try decoder.decode(BackendEnvelope.self, from: data)
}

print("OmxMenubarContractTests passed")
