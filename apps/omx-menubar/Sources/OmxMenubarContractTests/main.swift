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

let refresh = try decode("refresh.response.json")
assert(refresh.ok)
assert(refresh.data?.dashboard?.accounts.accounts.isEmpty == true)

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

func decode(_ name: String) throws -> BackendEnvelope {
    let data = try Data(contentsOf: fixtureRoot.appendingPathComponent(name))
    return try decoder.decode(BackendEnvelope.self, from: data)
}

print("OmxMenubarContractTests passed")
