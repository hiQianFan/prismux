use crate::{
    AccountRef, Availability, ConfigProfile, CostStatus, OpenMuxError, PlatformInfo, Result,
    UsageDataQuality, UsageDiagnostic, UsageEvent, UsageScanWatermark, UsageSnapshot, UsageSource,
    UsageSourceFingerprint, UsageSummary, UsageSummaryQuery, UsageTokenBreakdown,
    storage::create_dir_private,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct StateStore {
    conn: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRecord {
    pub local_id: String,
    pub provider: String,
    pub display_number: u32,
    pub alias: Option<String>,
    pub provider_subject_kind: Option<String>,
    pub provider_subject_hash: Option<String>,
    pub provider_subject_label: Option<String>,
    pub account_label: Option<String>,
    pub plan_label: Option<String>,
    pub auth_type: Option<String>,
    pub expires_at_unix: Option<i64>,
    pub auth_hash: String,
    pub secret_ref: String,
    pub imported_at_unix: u64,
    pub last_activated_at_unix: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertAccount {
    pub provider: String,
    pub alias: Option<String>,
    pub provider_subject_kind: Option<String>,
    pub provider_subject_hash: Option<String>,
    pub provider_subject_label: Option<String>,
    pub account_label: Option<String>,
    pub plan_label: Option<String>,
    pub auth_type: Option<String>,
    pub expires_at_unix: Option<i64>,
    pub auth_hash: String,
    pub secret_ref: String,
    pub imported_at_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSubjectUpdate<'a> {
    pub local_id: &'a str,
    pub subject_kind: &'a str,
    pub subject_hash: &'a str,
    pub subject_label: &'a str,
    pub account_label: Option<&'a str>,
    pub plan_label: Option<&'a str>,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRecord {
    pub local_id: String,
    pub provider: String,
    pub display_number: Option<u32>,
    pub name: String,
    pub label: Option<String>,
    pub profile_kind: String,
    pub provider_id: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub auth_type: Option<String>,
    pub config_hash: String,
    pub secret_ref: String,
    pub imported_at_unix: u64,
    pub last_activated_at_unix: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertProfile {
    pub provider: String,
    pub name: String,
    pub label: Option<String>,
    pub profile_kind: String,
    pub provider_id: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub auth_type: Option<String>,
    pub config_hash: String,
    pub secret_ref: String,
    pub imported_at_unix: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKindRecord {
    Account,
    Profile,
}

impl TargetKindRecord {
    fn as_str(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Profile => "profile",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct StoredUsageSnapshot {
    source: UsageSource,
    refreshed_at_unix: Option<i64>,
    summary: Availability,
    limits: Vec<crate::UsageLimit>,
    #[serde(default)]
    reset_credits: Option<crate::UsageResetCredits>,
}

#[derive(Debug, Clone, PartialEq)]
struct UsageEventPayload {
    client: String,
    model_provider: Option<String>,
    model: Option<String>,
    session_id: Option<String>,
    request_id: Option<String>,
    occurred_at_unix: i64,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    cache_write_5m_tokens: Option<u64>,
    cache_write_1h_tokens: Option<u64>,
    reasoning_tokens: u64,
    extra_tokens: u64,
    normalized_total_tokens: u64,
    provider_total_tokens: Option<u64>,
    estimated_cost_usd: Option<f64>,
    cost_status: String,
    quality: String,
}

impl UsageEventPayload {
    fn from_event(event: &UsageEvent) -> Self {
        Self {
            client: event.client.clone(),
            model_provider: event.model_provider.clone(),
            model: event.model.clone(),
            session_id: event.session_id.clone(),
            request_id: event.request_id.clone(),
            occurred_at_unix: event.occurred_at_unix,
            input_tokens: event.tokens.input,
            output_tokens: event.tokens.output,
            cache_read_tokens: event.tokens.cache_read,
            cache_write_tokens: event.tokens.cache_write,
            cache_write_5m_tokens: event.tokens.cache_write_5m,
            cache_write_1h_tokens: event.tokens.cache_write_1h,
            reasoning_tokens: event.tokens.reasoning,
            extra_tokens: event.tokens.extra,
            normalized_total_tokens: event.normalized_total_tokens(),
            provider_total_tokens: event.provider_total_tokens,
            estimated_cost_usd: event.estimated_cost_usd,
            cost_status: cost_status_name(&event.cost_status).to_string(),
            quality: usage_quality_name(&event.quality).to_string(),
        }
    }

    fn matches_except_cost(&self, other: &Self) -> bool {
        self.client == other.client
            && self.model_provider == other.model_provider
            && self.model == other.model
            && self.session_id == other.session_id
            && self.request_id == other.request_id
            && self.occurred_at_unix == other.occurred_at_unix
            && self.input_tokens == other.input_tokens
            && self.output_tokens == other.output_tokens
            && self.cache_read_tokens == other.cache_read_tokens
            && self.cache_write_tokens == other.cache_write_tokens
            && self.cache_write_5m_tokens == other.cache_write_5m_tokens
            && self.cache_write_1h_tokens == other.cache_write_1h_tokens
            && self.reasoning_tokens == other.reasoning_tokens
            && self.extra_tokens == other.extra_tokens
            && self.normalized_total_tokens == other.normalized_total_tokens
            && self.provider_total_tokens == other.provider_total_tokens
            && self.quality == other.quality
    }
}

impl StateStore {
    pub fn open(state_root: &Path) -> Result<Self> {
        create_dir_private(state_root)?;
        let path = state_root.join("omx-state.sqlite");
        let conn = Connection::open(&path).map_err(|err| db_error(&path, err))?;
        conn.busy_timeout(std::time::Duration::from_millis(2500))
            .map_err(|err| db_error(&path, err))?;
        let store = Self { conn };
        store.migrate(&path)?;
        Ok(store)
    }

    fn migrate(&self, path: &Path) -> Result<()> {
        self.conn
            .execute_batch(
                r#"
                PRAGMA foreign_keys = ON;
                PRAGMA journal_mode = WAL;
                CREATE TABLE IF NOT EXISTS accounts (
                    local_id TEXT PRIMARY KEY,
                    provider TEXT NOT NULL,
                    display_number INTEGER NOT NULL,
                    alias TEXT,
                    provider_subject_kind TEXT,
                    provider_subject_hash TEXT,
                    provider_subject_label TEXT,
                    account_label TEXT,
                    plan_label TEXT,
                    auth_type TEXT,
                    expires_at_unix INTEGER,
                    auth_hash TEXT NOT NULL,
                    secret_ref TEXT NOT NULL,
                    imported_at_unix INTEGER NOT NULL,
                    updated_at_unix INTEGER NOT NULL,
                    last_activated_at_unix INTEGER,
                    archived_at_unix INTEGER,
                    UNIQUE(provider, display_number),
                    UNIQUE(provider, auth_hash)
                );
                CREATE TABLE IF NOT EXISTS profiles (
                    local_id TEXT PRIMARY KEY,
                    provider TEXT NOT NULL,
                    display_number INTEGER,
                    name TEXT NOT NULL,
                    label TEXT,
                    profile_kind TEXT NOT NULL,
                    provider_id TEXT,
                    base_url TEXT,
                    model TEXT,
                    auth_type TEXT,
                    config_hash TEXT NOT NULL,
                    secret_ref TEXT NOT NULL,
                    imported_at_unix INTEGER NOT NULL,
                    updated_at_unix INTEGER NOT NULL,
                    last_activated_at_unix INTEGER,
                    archived_at_unix INTEGER,
                    UNIQUE(provider, name),
                    UNIQUE(provider, config_hash)
                );
                CREATE TABLE IF NOT EXISTS active_targets (
                    provider TEXT NOT NULL,
                    target_kind TEXT NOT NULL,
                    local_id TEXT NOT NULL,
                    previous_local_id TEXT,
                    activated_at_unix INTEGER NOT NULL,
                    PRIMARY KEY(provider, target_kind)
                );
                CREATE TABLE IF NOT EXISTS quota_snapshots (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    local_id TEXT NOT NULL,
                    provider TEXT NOT NULL,
                    captured_at_unix INTEGER NOT NULL,
                    source TEXT NOT NULL,
                    snapshot_json TEXT NOT NULL,
                    diagnostic_json TEXT
                );
                CREATE TABLE IF NOT EXISTS refresh_attempts (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    local_id TEXT NOT NULL,
                    provider TEXT NOT NULL,
                    refresh_kind TEXT NOT NULL,
                    trigger TEXT NOT NULL,
                    attempted_at_unix INTEGER NOT NULL,
                    completed_at_unix INTEGER,
                    status TEXT NOT NULL,
                    error_code TEXT,
                    error_message TEXT,
                    used_snapshot_id INTEGER
                );
                CREATE TABLE IF NOT EXISTS usage_events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    client TEXT NOT NULL,
                    model_provider TEXT,
                    model TEXT,
                    session_id TEXT,
                    request_id TEXT,
                    project_path TEXT,
                    occurred_at_unix INTEGER NOT NULL,
                    input_tokens INTEGER NOT NULL DEFAULT 0,
                    output_tokens INTEGER NOT NULL DEFAULT 0,
                    cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                    cache_write_tokens INTEGER NOT NULL DEFAULT 0,
                    cache_write_5m_tokens INTEGER,
                    cache_write_1h_tokens INTEGER,
                    reasoning_tokens INTEGER NOT NULL DEFAULT 0,
                    extra_tokens INTEGER NOT NULL DEFAULT 0,
                    normalized_total_tokens INTEGER NOT NULL DEFAULT 0,
                    provider_total_tokens INTEGER,
                    estimated_cost_usd REAL,
                    cost_status TEXT NOT NULL,
                    source_kind TEXT NOT NULL,
                    source_path TEXT,
                    source_fingerprint_json TEXT,
                    source_offset INTEGER,
                    source_record_id TEXT,
                    source_record_hash TEXT,
                    backend TEXT NOT NULL,
                    backend_version TEXT NOT NULL,
                    parser_schema_version INTEGER NOT NULL,
                    quality TEXT NOT NULL,
                    event_hash TEXT NOT NULL UNIQUE,
                    ingested_at_unix INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS scan_watermarks (
                    source_id TEXT PRIMARY KEY,
                    client TEXT NOT NULL,
                    backend TEXT NOT NULL,
                    backend_version TEXT NOT NULL,
                    parser_schema_version INTEGER NOT NULL,
                    source_kind TEXT NOT NULL,
                    source_path TEXT NOT NULL,
                    source_fingerprint_json TEXT NOT NULL,
                    last_offset INTEGER,
                    last_record_id TEXT,
                    last_scanned_at_unix INTEGER NOT NULL,
                    last_scan_status TEXT NOT NULL,
                    diagnostic_code TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_accounts_provider_active
                    ON accounts(provider, archived_at_unix, display_number);
                CREATE INDEX IF NOT EXISTS idx_profiles_provider_active
                    ON profiles(provider, archived_at_unix, display_number, name);
                CREATE INDEX IF NOT EXISTS idx_quota_latest
                    ON quota_snapshots(local_id, captured_at_unix DESC);
                CREATE INDEX IF NOT EXISTS idx_refresh_latest
                    ON refresh_attempts(local_id, attempted_at_unix DESC);
                CREATE INDEX IF NOT EXISTS idx_usage_client_time
                    ON usage_events(client, occurred_at_unix DESC);
                CREATE INDEX IF NOT EXISTS idx_usage_client_model_time
                    ON usage_events(client, model, occurred_at_unix DESC);
                CREATE INDEX IF NOT EXISTS idx_usage_client_provider_time
                    ON usage_events(client, model_provider, occurred_at_unix DESC);
                CREATE INDEX IF NOT EXISTS idx_usage_client_project_time
                    ON usage_events(client, project_path, occurred_at_unix DESC);
                CREATE INDEX IF NOT EXISTS idx_usage_session
                    ON usage_events(client, session_id, occurred_at_unix DESC);
                "#,
            )
            .map_err(|err| db_error(path, err))?;
        self.add_column_if_missing(path, "accounts", "provider_subject_kind", "TEXT")?;
        self.add_column_if_missing(path, "accounts", "provider_subject_hash", "TEXT")?;
        self.add_column_if_missing(path, "accounts", "provider_subject_label", "TEXT")?;
        self.add_column_if_missing(path, "usage_events", "source_record_hash", "TEXT")?;
        self.delete_duplicate_active_targets()?;
        if self.delete_archived_targets()? {
            self.compact_display_numbers()?;
        }
        self.conn
            .execute(
                r#"
                CREATE UNIQUE INDEX IF NOT EXISTS idx_accounts_provider_subject
                    ON accounts(provider, provider_subject_kind, provider_subject_hash)
                    WHERE provider_subject_hash IS NOT NULL
                "#,
                [],
            )
            .map_err(|err| db_error(path, err))?;
        Ok(())
    }

    pub fn upsert_account(&self, input: UpsertAccount) -> Result<AccountRecord> {
        let subject_existing = if let (Some(kind), Some(hash)) = (
            input.provider_subject_kind.as_deref(),
            input.provider_subject_hash.as_deref(),
        ) {
            self.account_by_subject(&input.provider, kind, hash)?
        } else {
            None
        };
        let auth_existing = self.account_by_auth_hash(&input.provider, &input.auth_hash)?;
        let existing = match (subject_existing, auth_existing) {
            (Some(subject_account), Some(auth_account))
                if subject_account.local_id != auth_account.local_id =>
            {
                self.delete_duplicate_auth_account(&auth_account)?;
                Some(subject_account)
            }
            (Some(subject_account), _) => Some(subject_account),
            (None, auth_account) => auth_account,
        };

        if let Some(mut existing) = existing {
            self.conn
                .execute(
                    r#"
                    UPDATE accounts
                    SET alias = COALESCE(?1, alias),
                        provider_subject_kind = COALESCE(?2, provider_subject_kind),
                        provider_subject_hash = COALESCE(?3, provider_subject_hash),
                        provider_subject_label = COALESCE(?4, provider_subject_label),
                        account_label = COALESCE(?5, account_label),
                        plan_label = COALESCE(?6, plan_label),
                        auth_type = ?7,
                        expires_at_unix = ?8,
                        auth_hash = ?9,
                        secret_ref = ?10,
                        imported_at_unix = ?11,
                        updated_at_unix = ?11,
                        archived_at_unix = NULL
                    WHERE local_id = ?12
                    "#,
                    params![
                        input.alias,
                        input.provider_subject_kind,
                        input.provider_subject_hash,
                        input.provider_subject_label,
                        input.account_label,
                        input.plan_label,
                        input.auth_type,
                        input.expires_at_unix,
                        input.auth_hash,
                        input.secret_ref,
                        input.imported_at_unix,
                        existing.local_id,
                    ],
                )
                .map_err(db_error_no_path)?;
            existing = self
                .account_by_local_id(&existing.local_id)?
                .expect("updated account should exist");
            return Ok(existing);
        }

        let display_number = self.next_account_number(&input.provider)?;
        let local_id =
            self.next_local_id("accounts", &input.provider, "account", display_number)?;
        self.conn
            .execute(
                r#"
                INSERT INTO accounts (
                    local_id, provider, display_number, alias, provider_subject_kind,
                    provider_subject_hash, provider_subject_label, account_label, plan_label,
                    auth_type, expires_at_unix, auth_hash, secret_ref, imported_at_unix,
                    updated_at_unix
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)
                "#,
                params![
                    local_id,
                    input.provider,
                    display_number,
                    input.alias,
                    input.provider_subject_kind,
                    input.provider_subject_hash,
                    input.provider_subject_label,
                    input.account_label,
                    input.plan_label,
                    input.auth_type,
                    input.expires_at_unix,
                    input.auth_hash,
                    input.secret_ref,
                    input.imported_at_unix,
                ],
            )
            .map_err(db_error_no_path)?;
        Ok(self
            .account_by_local_id(&local_id)?
            .expect("inserted account should exist"))
    }

    pub fn list_accounts(&self, provider: &str) -> Result<Vec<AccountRecord>> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "{} WHERE provider = ?1 AND archived_at_unix IS NULL ORDER BY display_number ASC",
                account_select()
            ))
            .map_err(db_error_no_path)?;
        let rows = stmt
            .query_map(params![provider], account_from_row)
            .map_err(db_error_no_path)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db_error_no_path)
    }

    pub fn account_by_selector(
        &self,
        provider: &str,
        selector: &str,
    ) -> Result<Option<AccountRecord>> {
        if let Some(account) = self.account_by_local_id(selector)?
            && account.provider == provider
        {
            return Ok(Some(account));
        }
        if let Ok(number) = selector.parse::<u32>() {
            return self.account_by_number(provider, number);
        }
        self.conn
            .query_row(
                &format!(
                    "{} WHERE provider = ?1 AND alias = ?2 AND archived_at_unix IS NULL",
                    account_select()
                ),
                params![provider, selector],
                account_from_row,
            )
            .optional()
            .map_err(db_error_no_path)
    }

    pub fn set_active_account(&self, provider: &str, local_id: &str, now: u64) -> Result<()> {
        self.set_active_account_preserving_profile(provider, local_id, now)?;
        Ok(())
    }

    pub fn set_active_account_preserving_profile(
        &self,
        provider: &str,
        local_id: &str,
        now: u64,
    ) -> Result<()> {
        self.set_active_target(provider, TargetKindRecord::Account, local_id, now)?;
        self.conn
            .execute(
                "UPDATE accounts SET last_activated_at_unix = ?1 WHERE local_id = ?2",
                params![now, local_id],
            )
            .map_err(db_error_no_path)?;
        Ok(())
    }

    pub fn active_account(&self, provider: &str) -> Result<Option<AccountRecord>> {
        let Some(local_id) = self.active_local_id(provider, TargetKindRecord::Account)? else {
            return Ok(None);
        };
        self.account_by_local_id(&local_id)
    }

    pub fn remove_account(&self, local_id: &str) -> Result<()> {
        let provider = self
            .account_by_local_id(local_id)?
            .map(|account| account.provider);
        self.clear_active_local_id(local_id)?;
        self.conn
            .execute(
                "DELETE FROM quota_snapshots WHERE local_id = ?1",
                params![local_id],
            )
            .map_err(db_error_no_path)?;
        self.conn
            .execute(
                "DELETE FROM refresh_attempts WHERE local_id = ?1",
                params![local_id],
            )
            .map_err(db_error_no_path)?;
        self.conn
            .execute(
                "DELETE FROM accounts WHERE local_id = ?1",
                params![local_id],
            )
            .map_err(db_error_no_path)?;
        if let Some(provider) = provider {
            self.compact_table_display_numbers_for_provider("accounts", &provider)?;
        }
        Ok(())
    }

    pub fn set_account_alias(&self, local_id: &str, alias: &str, now: u64) -> Result<()> {
        let updated = self
            .conn
            .execute(
                "UPDATE accounts SET alias = ?1, updated_at_unix = ?2 WHERE local_id = ?3",
                params![alias, now, local_id],
            )
            .map_err(db_error_no_path)?;
        if updated != 1 {
            return Err(OpenMuxError::AccountNotFound {
                platform: "unknown".to_string(),
                account: local_id.to_string(),
            });
        }
        Ok(())
    }

    pub fn set_account_subject(&self, input: AccountSubjectUpdate<'_>) -> Result<()> {
        let updated = self
            .conn
            .execute(
                r#"
                UPDATE accounts
                SET provider_subject_kind = ?1,
                    provider_subject_hash = ?2,
                    provider_subject_label = ?3,
                    account_label = COALESCE(?4, account_label),
                    plan_label = COALESCE(?5, plan_label),
                    updated_at_unix = ?6
                WHERE local_id = ?7
                "#,
                params![
                    input.subject_kind,
                    input.subject_hash,
                    input.subject_label,
                    input.account_label,
                    input.plan_label,
                    input.updated_at_unix,
                    input.local_id,
                ],
            )
            .map_err(db_error_no_path)?;
        if updated != 1 {
            return Err(OpenMuxError::AccountNotFound {
                platform: "unknown".to_string(),
                account: input.local_id.to_string(),
            });
        }
        Ok(())
    }

    pub fn update_account_auth(
        &self,
        provider: &str,
        local_id: &str,
        auth_hash: &str,
        secret_ref: &str,
        now: u64,
    ) -> Result<AccountRecord> {
        let updated = self
            .conn
            .execute(
                r#"
                UPDATE accounts
                SET auth_hash = ?1,
                    secret_ref = ?2,
                    imported_at_unix = ?3,
                    updated_at_unix = ?3
                WHERE local_id = ?4
                  AND provider = ?5
                  AND archived_at_unix IS NULL
                "#,
                params![auth_hash, secret_ref, now, local_id, provider],
            )
            .map_err(db_error_no_path)?;
        if updated != 1 {
            return Err(OpenMuxError::AccountNotFound {
                platform: provider.to_string(),
                account: local_id.to_string(),
            });
        }
        self.account_by_local_id(local_id)?
            .ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: provider.to_string(),
                account: local_id.to_string(),
            })
    }

    pub fn merge_account_into(&self, keep_local_id: &str, remove_local_id: &str) -> Result<()> {
        let transaction = self
            .conn
            .unchecked_transaction()
            .map_err(db_error_no_path)?;
        let account_details = |local_id: &str| {
            transaction
                .query_row(
                    r#"
                    SELECT provider, auth_hash, secret_ref, imported_at_unix,
                           EXISTS(
                               SELECT 1 FROM active_targets
                               WHERE active_targets.provider = accounts.provider
                                 AND active_targets.target_kind = 'account'
                                 AND active_targets.local_id = accounts.local_id
                           )
                    FROM accounts
                    WHERE local_id = ?1
                    "#,
                    params![local_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, u64>(3)?,
                            row.get::<_, bool>(4)?,
                        ))
                    },
                )
                .optional()
                .map_err(db_error_no_path)
        };
        let keep =
            account_details(keep_local_id)?.ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: "unknown".to_string(),
                account: keep_local_id.to_string(),
            })?;
        let remove =
            account_details(remove_local_id)?.ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: keep.0.clone(),
                account: remove_local_id.to_string(),
            })?;
        if keep.0 != remove.0 {
            return Err(OpenMuxError::Message(format!(
                "cannot merge accounts from different providers: `{}` and `{}`",
                keep.0, remove.0
            )));
        }
        if keep_local_id == remove_local_id {
            return Ok(());
        }
        let keep_provider = keep.0.clone();
        let removed_credentials_win =
            (remove.4 && !keep.4) || (remove.4 == keep.4 && remove.3 > keep.3);
        let (auth_hash, secret_ref, auth_updated_at) = if removed_credentials_win {
            (remove.1, remove.2, remove.3)
        } else {
            (keep.1, keep.2, keep.3)
        };

        transaction
            .execute(
                "UPDATE active_targets SET local_id = ?1 WHERE local_id = ?2",
                params![keep_local_id, remove_local_id],
            )
            .map_err(db_error_no_path)?;
        transaction
            .execute(
                "UPDATE active_targets SET previous_local_id = ?1 WHERE previous_local_id = ?2",
                params![keep_local_id, remove_local_id],
            )
            .map_err(db_error_no_path)?;
        transaction
            .execute(
                "UPDATE quota_snapshots SET local_id = ?1 WHERE local_id = ?2",
                params![keep_local_id, remove_local_id],
            )
            .map_err(db_error_no_path)?;
        transaction
            .execute(
                "UPDATE refresh_attempts SET local_id = ?1 WHERE local_id = ?2",
                params![keep_local_id, remove_local_id],
            )
            .map_err(db_error_no_path)?;
        transaction
            .execute(
                "DELETE FROM accounts WHERE local_id = ?1",
                params![remove_local_id],
            )
            .map_err(db_error_no_path)?;
        transaction
            .execute(
                "UPDATE accounts SET auth_hash = ?1, secret_ref = ?2, imported_at_unix = ?3, updated_at_unix = MAX(updated_at_unix, ?3) WHERE local_id = ?4",
                params![auth_hash, secret_ref, auth_updated_at, keep_local_id],
            )
            .map_err(db_error_no_path)?;

        let local_ids = {
            let mut statement = transaction
                .prepare(
                    "SELECT local_id FROM accounts WHERE provider = ?1 AND display_number IS NOT NULL ORDER BY display_number ASC, imported_at_unix ASC, local_id ASC",
                )
                .map_err(db_error_no_path)?;
            statement
                .query_map(params![keep_provider], |row| row.get::<_, String>(0))
                .map_err(db_error_no_path)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(db_error_no_path)?
        };
        for (index, local_id) in local_ids.into_iter().enumerate() {
            transaction
                .execute(
                    "UPDATE accounts SET display_number = ?1 WHERE local_id = ?2",
                    params![(index as u32) + 1, local_id],
                )
                .map_err(db_error_no_path)?;
        }
        transaction.commit().map_err(db_error_no_path)?;
        Ok(())
    }

    pub fn clear_account_alias(&self, local_id: &str, now: u64) -> Result<()> {
        self.conn
            .execute(
                "UPDATE accounts SET alias = NULL, updated_at_unix = ?1 WHERE local_id = ?2",
                params![now, local_id],
            )
            .map_err(db_error_no_path)?;
        Ok(())
    }

    pub fn clear_account_alias_by_selector(
        &self,
        provider: &str,
        selector: &str,
        now: u64,
    ) -> Result<AccountRef> {
        let account = self
            .account_by_selector(provider, selector)?
            .ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: provider.to_string(),
                account: selector.to_string(),
            })?;
        self.clear_account_alias(&account.local_id, now)?;
        Ok(AccountRef {
            platform: provider.to_string(),
            local_id: account.local_id,
            number: account.display_number,
            alias: None,
        })
    }

    pub fn upsert_profile(&self, input: UpsertProfile) -> Result<ProfileRecord> {
        if let Some(existing) = self.profile_by_name_any(&input.provider, &input.name)? {
            self.conn
                .execute(
                    r#"
                    UPDATE profiles
                    SET label = ?1, profile_kind = ?2, provider_id = ?3, base_url = ?4,
                        model = ?5, auth_type = ?6, config_hash = ?7, secret_ref = ?8,
                        imported_at_unix = ?9, updated_at_unix = ?9, archived_at_unix = NULL
                    WHERE local_id = ?10
                    "#,
                    params![
                        input.label,
                        input.profile_kind,
                        input.provider_id,
                        input.base_url,
                        input.model,
                        input.auth_type,
                        input.config_hash,
                        input.secret_ref,
                        input.imported_at_unix,
                        existing.local_id,
                    ],
                )
                .map_err(db_error_no_path)?;
            return Ok(self
                .profile_by_local_id(&existing.local_id)?
                .expect("updated profile should exist"));
        }

        let display_number = self.next_profile_number(&input.provider)?;
        let local_id =
            self.next_local_id("profiles", &input.provider, "profile", display_number)?;
        self.conn
            .execute(
                r#"
                INSERT INTO profiles (
                    local_id, provider, display_number, name, label, profile_kind,
                    provider_id, base_url, model, auth_type, config_hash, secret_ref,
                    imported_at_unix, updated_at_unix
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
                "#,
                params![
                    local_id,
                    input.provider,
                    display_number,
                    input.name,
                    input.label,
                    input.profile_kind,
                    input.provider_id,
                    input.base_url,
                    input.model,
                    input.auth_type,
                    input.config_hash,
                    input.secret_ref,
                    input.imported_at_unix,
                ],
            )
            .map_err(db_error_no_path)?;
        Ok(self
            .profile_by_local_id(&local_id)?
            .expect("inserted profile should exist"))
    }

    pub fn list_profiles(&self, provider: &str) -> Result<Vec<ProfileRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT local_id, provider, display_number, name, label, profile_kind,
                       provider_id, base_url, model, auth_type, config_hash, secret_ref,
                       imported_at_unix, last_activated_at_unix
                FROM profiles
                WHERE provider = ?1 AND archived_at_unix IS NULL
                ORDER BY display_number ASC, name ASC
                "#,
            )
            .map_err(db_error_no_path)?;
        let rows = stmt
            .query_map(params![provider], profile_from_row)
            .map_err(db_error_no_path)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db_error_no_path)
    }

    pub fn profile_by_selector(
        &self,
        provider: &str,
        selector: &str,
    ) -> Result<Option<ProfileRecord>> {
        if let Some(profile) = self.profile_by_local_id(selector)?
            && profile.provider == provider
        {
            return Ok(Some(profile));
        }
        if let Ok(number) = selector.parse::<u32>() {
            return self.profile_by_number(provider, number);
        }
        self.profile_by_name(provider, selector)
    }

    pub fn set_active_profile(&self, provider: &str, local_id: &str, now: u64) -> Result<()> {
        self.set_active_profile_preserving_account(provider, local_id, now)?;
        Ok(())
    }

    pub fn set_active_profile_preserving_account(
        &self,
        provider: &str,
        local_id: &str,
        now: u64,
    ) -> Result<()> {
        self.set_active_target(provider, TargetKindRecord::Profile, local_id, now)?;
        self.conn
            .execute(
                "UPDATE profiles SET last_activated_at_unix = ?1 WHERE local_id = ?2",
                params![now, local_id],
            )
            .map_err(db_error_no_path)?;
        Ok(())
    }

    pub fn active_profile(&self, provider: &str) -> Result<Option<ProfileRecord>> {
        let Some(local_id) = self.active_local_id(provider, TargetKindRecord::Profile)? else {
            return Ok(None);
        };
        self.profile_by_local_id(&local_id)
    }

    pub fn remove_profile(&self, local_id: &str) -> Result<()> {
        let provider = self
            .profile_by_local_id(local_id)?
            .map(|profile| profile.provider);
        self.clear_active_local_id(local_id)?;
        self.conn
            .execute(
                "DELETE FROM profiles WHERE local_id = ?1",
                params![local_id],
            )
            .map_err(db_error_no_path)?;
        if let Some(provider) = provider {
            self.compact_table_display_numbers_for_provider("profiles", &provider)?;
        }
        Ok(())
    }

    pub fn save_quota_snapshot(
        &self,
        local_id: &str,
        provider: &str,
        usage: &UsageSnapshot,
    ) -> Result<()> {
        let snapshot = StoredUsageSnapshot {
            source: usage.source.clone(),
            refreshed_at_unix: usage.refreshed_at_unix,
            summary: usage.summary.clone(),
            limits: usage.limits.clone(),
            reset_credits: usage.reset_credits.clone(),
        };
        let snapshot_json = serde_json::to_string(&snapshot)
            .map_err(|err| OpenMuxError::Message(format!("encode quota snapshot: {err}")))?;
        let diagnostic_json = serde_json::to_string(&usage.diagnostics)
            .map_err(|err| OpenMuxError::Message(format!("encode quota diagnostics: {err}")))?;
        self.conn
            .execute(
                r#"
                INSERT INTO quota_snapshots
                    (local_id, provider, captured_at_unix, source, snapshot_json, diagnostic_json)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    local_id,
                    provider,
                    usage.refreshed_at_unix.unwrap_or_default(),
                    usage_source_name(&usage.source),
                    snapshot_json,
                    diagnostic_json,
                ],
            )
            .map_err(db_error_no_path)?;
        Ok(())
    }

    pub fn latest_quota_snapshot(&self, local_id: &str) -> Result<Option<UsageSnapshot>> {
        self.conn
            .query_row(
                r#"
                SELECT snapshot_json, diagnostic_json
                FROM quota_snapshots
                WHERE local_id = ?1
                ORDER BY captured_at_unix DESC, id DESC
                LIMIT 1
                "#,
                params![local_id],
                |row| {
                    let snapshot_json: String = row.get(0)?;
                    let diagnostic_json: Option<String> = row.get(1)?;
                    let snapshot: StoredUsageSnapshot = serde_json::from_str(&snapshot_json)
                        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
                    let diagnostics = diagnostic_json
                        .as_deref()
                        .map(serde_json::from_str)
                        .transpose()
                        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?
                        .unwrap_or_default();
                    Ok(UsageSnapshot {
                        source: snapshot.source,
                        refreshed_at_unix: snapshot.refreshed_at_unix,
                        summary: snapshot.summary,
                        limits: snapshot.limits,
                        reset_credits: snapshot.reset_credits,
                        diagnostics,
                    })
                },
            )
            .optional()
            .map_err(db_error_no_path)
    }

    pub fn record_refresh_attempt(
        &self,
        local_id: &str,
        provider: &str,
        status: &str,
        error: Option<&UsageDiagnostic>,
        now: u64,
    ) -> Result<()> {
        self.conn
            .execute(
                r#"
                INSERT INTO refresh_attempts (
                    local_id, provider, refresh_kind, trigger, attempted_at_unix,
                    completed_at_unix, status, error_code, error_message
                )
                VALUES (?1, ?2, 'interactive', 'list', ?3, ?3, ?4, ?5, ?6)
                "#,
                params![
                    local_id,
                    provider,
                    now,
                    status,
                    error.map(|err| err.code.as_str()),
                    error.map(|err| err.message.as_str()),
                ],
            )
            .map_err(db_error_no_path)?;
        Ok(())
    }

    pub fn insert_usage_event(&self, event: &UsageEvent, ingested_at_unix: u64) -> Result<bool> {
        self.insert_usage_event_on_connection(&self.conn, event, ingested_at_unix)
    }

    pub fn ingest_usage_events(
        &self,
        events: &[UsageEvent],
        watermark: Option<&UsageScanWatermark>,
        ingested_at_unix: u64,
    ) -> Result<usize> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(db_error_no_path)?;
        let mut inserted = 0;
        for event in events {
            if self.insert_usage_event_on_connection(&tx, event, ingested_at_unix)? {
                inserted += 1;
            }
        }
        if let Some(watermark) = watermark {
            self.update_scan_watermark_on_connection(&tx, watermark)?;
        }
        tx.commit().map_err(db_error_no_path)?;
        Ok(inserted)
    }

    fn insert_usage_event_on_connection(
        &self,
        conn: &Connection,
        event: &UsageEvent,
        ingested_at_unix: u64,
    ) -> Result<bool> {
        if let Some(existing) = self.usage_event_payload_on_connection(conn, &event.event_hash)? {
            let incoming = UsageEventPayload::from_event(event);
            if existing != incoming {
                if existing.matches_except_cost(&incoming) {
                    return Ok(false);
                }
                return Err(OpenMuxError::Message(format!(
                    "usage event hash conflict: {}",
                    event.event_hash
                )));
            }
            return Ok(false);
        }

        conn.execute(
            r#"
                INSERT INTO usage_events (
                    client, model_provider, model, session_id, request_id, project_path,
                    occurred_at_unix, input_tokens, output_tokens, cache_read_tokens,
                    cache_write_tokens, cache_write_5m_tokens, cache_write_1h_tokens,
                    reasoning_tokens, extra_tokens, normalized_total_tokens,
                    provider_total_tokens, estimated_cost_usd, cost_status, source_kind,
                    source_path, source_fingerprint_json, source_offset, source_record_id,
                    source_record_hash, backend, backend_version, parser_schema_version,
                    quality, event_hash, ingested_at_unix
                )
                VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                    ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                    ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
                    ?31
                )
                "#,
            params![
                event.client.as_str(),
                event.model_provider.as_deref(),
                event.model.as_deref(),
                event.session_id.as_deref(),
                event.request_id.as_deref(),
                event
                    .project_path
                    .as_ref()
                    .map(|path| display_path_string(path)),
                event.occurred_at_unix,
                event.tokens.input,
                event.tokens.output,
                event.tokens.cache_read,
                event.tokens.cache_write,
                event.tokens.cache_write_5m,
                event.tokens.cache_write_1h,
                event.tokens.reasoning,
                event.tokens.extra,
                event.normalized_total_tokens(),
                event.provider_total_tokens,
                event.estimated_cost_usd,
                cost_status_name(&event.cost_status),
                event.source.kind.as_str(),
                event
                    .source
                    .path
                    .as_ref()
                    .map(|path| display_path_string(path)),
                event.source.fingerprint_json.as_deref(),
                event.source.offset,
                event.source.record_id.as_deref(),
                event.source.record_hash.as_deref(),
                event.source.backend.as_str(),
                event.source.backend_version.as_str(),
                event.source.parser_schema_version,
                usage_quality_name(&event.quality),
                event.event_hash.as_str(),
                ingested_at_unix,
            ],
        )
        .map_err(db_error_no_path)?;
        Ok(true)
    }

    pub fn usage_summaries(
        &self,
        client: Option<&str>,
        since_unix: Option<i64>,
        until_unix: Option<i64>,
    ) -> Result<Vec<UsageSummary>> {
        self.usage_summaries_by(UsageSummaryQuery {
            client: client.map(str::to_string),
            since_unix,
            until_unix,
            ..UsageSummaryQuery::default()
        })
    }

    pub fn usage_summaries_by(&self, query: UsageSummaryQuery) -> Result<Vec<UsageSummary>> {
        let project_expr = "project_path";
        let local_day_expr = format!(
            "date(occurred_at_unix + {}, 'unixepoch')",
            query.local_day_offset_seconds
        );
        let select_local_day = if query.group_by_local_day {
            local_day_expr.as_str()
        } else {
            "NULL"
        };
        let local_hour_expr = format!(
            "strftime('%Y-%m-%dT%H', occurred_at_unix + {}, 'unixepoch')",
            query.local_day_offset_seconds
        );
        let select_local_hour = if query.group_by_local_hour {
            local_hour_expr.as_str()
        } else {
            "NULL"
        };
        let select_model_provider = if query.group_by_model_provider {
            "model_provider"
        } else {
            "NULL"
        };
        let select_model = if query.group_by_model {
            "model"
        } else {
            "NULL"
        };
        let select_project = if query.group_by_project {
            project_expr
        } else {
            "NULL"
        };
        let select_session = if query.group_by_session {
            "session_id"
        } else {
            "NULL"
        };
        let mut sql = format!(
            r#"
            SELECT client,
                   {select_local_day},
                   {select_model_provider},
                   {select_model},
                   {select_project},
                   {select_session},
                   SUM(input_tokens),
                   SUM(output_tokens),
                   SUM(cache_read_tokens),
                   SUM(cache_write_tokens),
                   SUM(cache_write_5m_tokens),
                   SUM(cache_write_1h_tokens),
                   SUM(reasoning_tokens),
                   SUM(extra_tokens),
                   SUM(normalized_total_tokens),
                   SUM(provider_total_tokens),
                   SUM(CASE WHEN estimated_cost_usd IS NOT NULL THEN estimated_cost_usd ELSE 0 END),
                   SUM(CASE WHEN estimated_cost_usd IS NOT NULL THEN 1 ELSE 0 END),
                   SUM(CASE WHEN cost_status = 'provider_reported' THEN 1 ELSE 0 END),
                   SUM(CASE WHEN cost_status = 'estimated' THEN 1 ELSE 0 END),
                   SUM(CASE WHEN cost_status = 'missing' THEN 1 ELSE 0 END),
                   COUNT(*),
                   {select_local_hour}
            FROM usage_events
            WHERE 1 = 1
            "#
        );
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(client) = query.client {
            sql.push_str(" AND client = ?");
            values.push(Box::new(client));
        }
        if let Some(since_unix) = query.since_unix {
            sql.push_str(" AND occurred_at_unix >= ?");
            values.push(Box::new(since_unix));
        }
        if let Some(until_unix) = query.until_unix {
            sql.push_str(" AND occurred_at_unix < ?");
            values.push(Box::new(until_unix));
        }
        if let Some(model_provider) = query.model_provider {
            sql.push_str(" AND model_provider = ?");
            values.push(Box::new(model_provider));
        }
        if let Some(model) = query.model {
            sql.push_str(" AND model = ?");
            values.push(Box::new(model));
        }
        if let Some(project_path) = query.project_path {
            sql.push_str(" AND project_path = ?");
            values.push(Box::new(display_path_string(&project_path)));
        }
        if let Some(session_id) = query.session_id {
            sql.push_str(" AND session_id = ?");
            values.push(Box::new(session_id));
        }

        sql.push_str(" GROUP BY client");
        if query.group_by_local_day {
            sql.push_str(", ");
            sql.push_str(&local_day_expr);
        }
        if query.group_by_local_hour {
            sql.push_str(", ");
            sql.push_str(&local_hour_expr);
        }
        if query.group_by_model_provider {
            sql.push_str(", model_provider");
        }
        if query.group_by_model {
            sql.push_str(", model");
        }
        if query.group_by_project {
            sql.push_str(", project_path");
        }
        if query.group_by_session {
            sql.push_str(", session_id");
        }
        sql.push_str(
            " ORDER BY client ASC, 2 ASC, model_provider ASC, model ASC, project_path ASC, session_id ASC",
        );

        let params = rusqlite::params_from_iter(values.iter().map(|value| value.as_ref()));
        let mut stmt = self.conn.prepare(&sql).map_err(db_error_no_path)?;
        let rows = stmt
            .query_map(params, usage_summary_from_row)
            .map_err(db_error_no_path)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db_error_no_path)
    }

    pub fn update_scan_watermark(&self, watermark: &UsageScanWatermark) -> Result<()> {
        self.update_scan_watermark_on_connection(&self.conn, watermark)
    }

    fn update_scan_watermark_on_connection(
        &self,
        conn: &Connection,
        watermark: &UsageScanWatermark,
    ) -> Result<()> {
        let fingerprint_json = watermark.source_fingerprint.to_json()?;
        conn.execute(
            r#"
                INSERT INTO scan_watermarks (
                    source_id, client, backend, backend_version, parser_schema_version,
                    source_kind, source_path, source_fingerprint_json, last_offset,
                    last_record_id, last_scanned_at_unix, last_scan_status, diagnostic_code
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                ON CONFLICT(source_id)
                DO UPDATE SET client = excluded.client,
                              backend = excluded.backend,
                              backend_version = excluded.backend_version,
                              parser_schema_version = excluded.parser_schema_version,
                              source_kind = excluded.source_kind,
                              source_path = excluded.source_path,
                              source_fingerprint_json = excluded.source_fingerprint_json,
                              last_offset = excluded.last_offset,
                              last_record_id = excluded.last_record_id,
                              last_scanned_at_unix = excluded.last_scanned_at_unix,
                              last_scan_status = excluded.last_scan_status,
                              diagnostic_code = excluded.diagnostic_code
                "#,
            params![
                watermark.source_id.as_str(),
                watermark.client.as_str(),
                watermark.backend.as_str(),
                watermark.backend_version.as_str(),
                watermark.parser_schema_version,
                watermark.source_kind.as_str(),
                display_path_string(&watermark.source_path),
                fingerprint_json,
                watermark.last_offset,
                watermark.last_record_id.as_deref(),
                watermark.last_scanned_at_unix,
                watermark.last_scan_status.as_str(),
                watermark.diagnostic_code.as_deref(),
            ],
        )
        .map_err(db_error_no_path)?;
        Ok(())
    }

    pub fn scan_watermark(&self, source_id: &str) -> Result<Option<UsageScanWatermark>> {
        self.conn
            .query_row(
                r#"
                SELECT source_id, client, backend, backend_version, parser_schema_version,
                       source_kind, source_path, source_fingerprint_json, last_offset,
                       last_record_id, last_scanned_at_unix, last_scan_status, diagnostic_code
                FROM scan_watermarks
                WHERE source_id = ?1
                "#,
                params![source_id],
                scan_watermark_from_row,
            )
            .optional()
            .map_err(db_error_no_path)
    }

    fn account_by_auth_hash(
        &self,
        provider: &str,
        auth_hash: &str,
    ) -> Result<Option<AccountRecord>> {
        self.conn
            .query_row(
                &format!(
                    "{} WHERE provider = ?1 AND auth_hash = ?2",
                    account_select()
                ),
                params![provider, auth_hash],
                account_from_row,
            )
            .optional()
            .map_err(db_error_no_path)
    }

    fn usage_event_payload_on_connection(
        &self,
        conn: &Connection,
        event_hash: &str,
    ) -> Result<Option<UsageEventPayload>> {
        conn.query_row(
            r#"
                SELECT client, model_provider, model, session_id, request_id,
                       occurred_at_unix, input_tokens, output_tokens, cache_read_tokens,
                       cache_write_tokens, cache_write_5m_tokens, cache_write_1h_tokens,
                       reasoning_tokens, extra_tokens, normalized_total_tokens,
                       provider_total_tokens, estimated_cost_usd, cost_status, quality
                FROM usage_events
                WHERE event_hash = ?1
                "#,
            params![event_hash],
            |row| {
                Ok(UsageEventPayload {
                    client: row.get(0)?,
                    model_provider: row.get(1)?,
                    model: row.get(2)?,
                    session_id: row.get(3)?,
                    request_id: row.get(4)?,
                    occurred_at_unix: row.get(5)?,
                    input_tokens: row.get(6)?,
                    output_tokens: row.get(7)?,
                    cache_read_tokens: row.get(8)?,
                    cache_write_tokens: row.get(9)?,
                    cache_write_5m_tokens: row.get(10)?,
                    cache_write_1h_tokens: row.get(11)?,
                    reasoning_tokens: row.get(12)?,
                    extra_tokens: row.get(13)?,
                    normalized_total_tokens: row.get(14)?,
                    provider_total_tokens: row.get(15)?,
                    estimated_cost_usd: row.get(16)?,
                    cost_status: row.get(17)?,
                    quality: row.get(18)?,
                })
            },
        )
        .optional()
        .map_err(db_error_no_path)
    }

    fn account_by_subject(
        &self,
        provider: &str,
        subject_kind: &str,
        subject_hash: &str,
    ) -> Result<Option<AccountRecord>> {
        self.conn
            .query_row(
                &format!(
                    "{} WHERE provider = ?1 AND provider_subject_kind = ?2 AND provider_subject_hash = ?3",
                    account_select()
                ),
                params![provider, subject_kind, subject_hash],
                account_from_row,
            )
            .optional()
            .map_err(db_error_no_path)
    }

    fn delete_duplicate_auth_account(&self, account: &AccountRecord) -> Result<()> {
        self.remove_account(&account.local_id)
    }

    pub fn account_by_local_id(&self, local_id: &str) -> Result<Option<AccountRecord>> {
        self.conn
            .query_row(
                &format!(
                    "{} WHERE local_id = ?1 AND archived_at_unix IS NULL",
                    account_select()
                ),
                params![local_id],
                account_from_row,
            )
            .optional()
            .map_err(db_error_no_path)
    }

    fn account_by_number(&self, provider: &str, number: u32) -> Result<Option<AccountRecord>> {
        self.conn
            .query_row(
                &format!(
                    "{} WHERE provider = ?1 AND display_number = ?2 AND archived_at_unix IS NULL",
                    account_select()
                ),
                params![provider, number],
                account_from_row,
            )
            .optional()
            .map_err(db_error_no_path)
    }

    fn next_account_number(&self, provider: &str) -> Result<u32> {
        self.next_available_display_number(
            "SELECT display_number FROM accounts WHERE provider = ?1 ORDER BY display_number ASC",
            provider,
        )
    }

    pub fn profile_by_local_id(&self, local_id: &str) -> Result<Option<ProfileRecord>> {
        let sql = profile_select("local_id = ?1 AND archived_at_unix IS NULL");
        self.conn
            .query_row(&sql, params![local_id], profile_from_row)
            .optional()
            .map_err(db_error_no_path)
    }

    fn profile_by_number(&self, provider: &str, number: u32) -> Result<Option<ProfileRecord>> {
        let sql =
            profile_select("provider = ?1 AND display_number = ?2 AND archived_at_unix IS NULL");
        self.conn
            .query_row(&sql, params![provider, number], profile_from_row)
            .optional()
            .map_err(db_error_no_path)
    }

    fn profile_by_name(&self, provider: &str, name: &str) -> Result<Option<ProfileRecord>> {
        let sql = profile_select("provider = ?1 AND name = ?2 AND archived_at_unix IS NULL");
        self.conn
            .query_row(&sql, params![provider, name], profile_from_row)
            .optional()
            .map_err(db_error_no_path)
    }

    fn profile_by_name_any(&self, provider: &str, name: &str) -> Result<Option<ProfileRecord>> {
        let sql = profile_select("provider = ?1 AND name = ?2");
        self.conn
            .query_row(&sql, params![provider, name], profile_from_row)
            .optional()
            .map_err(db_error_no_path)
    }

    fn next_profile_number(&self, provider: &str) -> Result<u32> {
        self.next_available_display_number(
            "SELECT display_number FROM profiles WHERE provider = ?1 AND display_number IS NOT NULL ORDER BY display_number ASC",
            provider,
        )
    }

    fn next_available_display_number(&self, sql: &str, provider: &str) -> Result<u32> {
        let mut stmt = self.conn.prepare(sql).map_err(db_error_no_path)?;
        let rows = stmt
            .query_map(params![provider], |row| row.get::<_, u32>(0))
            .map_err(db_error_no_path)?;
        let mut expected = 1;
        for number in rows {
            let number = number.map_err(db_error_no_path)?;
            if number == expected {
                expected += 1;
            } else if number > expected {
                break;
            }
        }
        Ok(expected)
    }

    fn next_local_id(
        &self,
        table: &str,
        provider: &str,
        target_kind: &str,
        display_number: u32,
    ) -> Result<String> {
        let base = format!("{provider}_{target_kind}_{display_number}");
        if !self.local_id_exists(table, &base)? {
            return Ok(base);
        }

        let mut suffix = 2;
        loop {
            let candidate = format!("{base}_{suffix}");
            if !self.local_id_exists(table, &candidate)? {
                return Ok(candidate);
            }
            suffix += 1;
        }
    }

    fn local_id_exists(&self, table: &str, local_id: &str) -> Result<bool> {
        self.conn
            .query_row(
                &format!("SELECT 1 FROM {table} WHERE local_id = ?1 LIMIT 1"),
                params![local_id],
                |_| Ok(()),
            )
            .optional()
            .map(|value| value.is_some())
            .map_err(db_error_no_path)
    }

    fn delete_archived_targets(&self) -> Result<bool> {
        let archived_count = self.archived_target_count()?;
        self.conn
            .execute(
                r#"
                DELETE FROM active_targets
                WHERE local_id IN (
                    SELECT local_id FROM accounts WHERE archived_at_unix IS NOT NULL
                    UNION
                    SELECT local_id FROM profiles WHERE archived_at_unix IS NOT NULL
                )
                OR previous_local_id IN (
                    SELECT local_id FROM accounts WHERE archived_at_unix IS NOT NULL
                    UNION
                    SELECT local_id FROM profiles WHERE archived_at_unix IS NOT NULL
                )
                "#,
                [],
            )
            .map_err(db_error_no_path)?;
        self.conn
            .execute(
                "DELETE FROM quota_snapshots WHERE local_id IN (SELECT local_id FROM accounts WHERE archived_at_unix IS NOT NULL)",
                [],
            )
            .map_err(db_error_no_path)?;
        self.conn
            .execute(
                "DELETE FROM refresh_attempts WHERE local_id IN (SELECT local_id FROM accounts WHERE archived_at_unix IS NOT NULL)",
                [],
            )
            .map_err(db_error_no_path)?;
        self.conn
            .execute(
                "DELETE FROM accounts WHERE archived_at_unix IS NOT NULL",
                [],
            )
            .map_err(db_error_no_path)?;
        self.conn
            .execute(
                "DELETE FROM profiles WHERE archived_at_unix IS NOT NULL",
                [],
            )
            .map_err(db_error_no_path)?;
        Ok(archived_count > 0)
    }

    fn archived_target_count(&self) -> Result<u64> {
        self.conn
            .query_row(
                r#"
                SELECT
                    (SELECT COUNT(*) FROM accounts WHERE archived_at_unix IS NOT NULL) +
                    (SELECT COUNT(*) FROM profiles WHERE archived_at_unix IS NOT NULL)
                "#,
                [],
                |row| row.get(0),
            )
            .map_err(db_error_no_path)
    }

    fn compact_display_numbers(&self) -> Result<()> {
        self.compact_table_display_numbers("accounts")?;
        self.compact_table_display_numbers("profiles")
    }

    fn compact_table_display_numbers(&self, table: &str) -> Result<()> {
        let mut providers_stmt = self
            .conn
            .prepare(&format!("SELECT DISTINCT provider FROM {table}"))
            .map_err(db_error_no_path)?;
        let providers = providers_stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(db_error_no_path)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db_error_no_path)?;
        drop(providers_stmt);

        for provider in providers {
            self.compact_table_display_numbers_for_provider(table, &provider)?;
        }
        Ok(())
    }

    fn compact_table_display_numbers_for_provider(
        &self,
        table: &str,
        provider: &str,
    ) -> Result<()> {
        let mut rows_stmt = self
            .conn
            .prepare(&format!(
                "SELECT local_id FROM {table} WHERE provider = ?1 AND display_number IS NOT NULL ORDER BY display_number ASC, imported_at_unix ASC, local_id ASC"
            ))
            .map_err(db_error_no_path)?;
        let local_ids = rows_stmt
            .query_map(params![provider], |row| row.get::<_, String>(0))
            .map_err(db_error_no_path)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db_error_no_path)?;
        drop(rows_stmt);

        for (index, local_id) in local_ids.into_iter().enumerate() {
            self.conn
                .execute(
                    &format!("UPDATE {table} SET display_number = ?1 WHERE local_id = ?2"),
                    params![(index as u32) + 1, local_id],
                )
                .map_err(db_error_no_path)?;
        }
        Ok(())
    }

    fn set_active_target(
        &self,
        provider: &str,
        kind: TargetKindRecord,
        local_id: &str,
        now: u64,
    ) -> Result<()> {
        let previous = self.active_any_local_id(provider)?;
        self.conn
            .execute(
                "DELETE FROM active_targets WHERE provider = ?1",
                params![provider],
            )
            .map_err(db_error_no_path)?;
        self.conn
            .execute(
                r#"
                INSERT INTO active_targets
                    (provider, target_kind, local_id, previous_local_id, activated_at_unix)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(provider, target_kind)
                DO UPDATE SET local_id = excluded.local_id,
                              previous_local_id = excluded.previous_local_id,
                              activated_at_unix = excluded.activated_at_unix
                "#,
                params![provider, kind.as_str(), local_id, previous, now],
            )
            .map_err(db_error_no_path)?;
        Ok(())
    }

    fn active_any_local_id(&self, provider: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                r#"
                SELECT local_id
                FROM active_targets
                WHERE provider = ?1
                ORDER BY activated_at_unix DESC,
                         CASE target_kind WHEN 'account' THEN 0 ELSE 1 END
                LIMIT 1
                "#,
                params![provider],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error_no_path)
    }

    fn delete_duplicate_active_targets(&self) -> Result<()> {
        self.conn
            .execute(
                r#"
                DELETE FROM active_targets
                WHERE rowid NOT IN (
                    SELECT rowid
                    FROM (
                        SELECT rowid, provider,
                               ROW_NUMBER() OVER (
                                   PARTITION BY provider
                                   ORDER BY activated_at_unix DESC,
                                            CASE target_kind WHEN 'account' THEN 0 ELSE 1 END
                               ) AS rank
                        FROM active_targets
                    )
                    WHERE rank = 1
                )
                "#,
                [],
            )
            .map_err(db_error_no_path)?;
        Ok(())
    }

    fn clear_active_local_id(&self, local_id: &str) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM active_targets WHERE local_id = ?1 OR previous_local_id = ?1",
                params![local_id],
            )
            .map_err(db_error_no_path)?;
        Ok(())
    }

    fn active_local_id(&self, provider: &str, kind: TargetKindRecord) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT local_id FROM active_targets WHERE provider = ?1 AND target_kind = ?2",
                params![provider, kind.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error_no_path)
    }

    fn add_column_if_missing(
        &self,
        path: &Path,
        table: &str,
        column: &str,
        column_type: &str,
    ) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(|err| db_error(path, err))?;
        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|err| db_error(path, err))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|err| db_error(path, err))?;
        if columns.iter().any(|name| name == column) {
            return Ok(());
        }
        self.conn
            .execute(
                &format!("ALTER TABLE {table} ADD COLUMN {column} {column_type}"),
                [],
            )
            .map_err(|err| db_error(path, err))?;
        Ok(())
    }
}

impl ProfileRecord {
    pub fn to_config_profile(&self, platform: PlatformInfo, active: bool) -> ConfigProfile {
        ConfigProfile {
            platform,
            local_id: self.local_id.clone(),
            name: self.name.clone(),
            active,
            config_path: self.secret_ref.clone(),
            provider_id: self.provider_id.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            number: self.display_number,
            auth_type: self.auth_type.clone(),
        }
    }
}

fn account_select() -> &'static str {
    r#"
    SELECT local_id, provider, display_number, alias, provider_subject_kind,
           provider_subject_hash, provider_subject_label, account_label, plan_label,
           auth_type, expires_at_unix, auth_hash, secret_ref, imported_at_unix,
           last_activated_at_unix
    FROM accounts
    "#
}

fn account_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccountRecord> {
    Ok(AccountRecord {
        local_id: row.get(0)?,
        provider: row.get(1)?,
        display_number: row.get(2)?,
        alias: row.get(3)?,
        provider_subject_kind: row.get(4)?,
        provider_subject_hash: row.get(5)?,
        provider_subject_label: row.get(6)?,
        account_label: row.get(7)?,
        plan_label: row.get(8)?,
        auth_type: row.get(9)?,
        expires_at_unix: row.get(10)?,
        auth_hash: row.get(11)?,
        secret_ref: row.get(12)?,
        imported_at_unix: row.get(13)?,
        last_activated_at_unix: row.get(14)?,
    })
}

fn profile_select(where_clause: &str) -> String {
    format!(
        r#"
        SELECT local_id, provider, display_number, name, label, profile_kind,
               provider_id, base_url, model, auth_type, config_hash, secret_ref,
               imported_at_unix, last_activated_at_unix
        FROM profiles
        WHERE {where_clause}
        "#
    )
}

fn profile_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProfileRecord> {
    Ok(ProfileRecord {
        local_id: row.get(0)?,
        provider: row.get(1)?,
        display_number: row.get(2)?,
        name: row.get(3)?,
        label: row.get(4)?,
        profile_kind: row.get(5)?,
        provider_id: row.get(6)?,
        base_url: row.get(7)?,
        model: row.get(8)?,
        auth_type: row.get(9)?,
        config_hash: row.get(10)?,
        secret_ref: row.get(11)?,
        imported_at_unix: row.get(12)?,
        last_activated_at_unix: row.get(13)?,
    })
}

fn usage_source_name(source: &UsageSource) -> &'static str {
    match source {
        UsageSource::RemoteApi => "remote_api",
        UsageSource::LocalSession => "local_session",
        UsageSource::StoredSnapshot => "stored_snapshot",
        UsageSource::Unavailable => "unavailable",
    }
}

fn cost_status_name(status: &CostStatus) -> &'static str {
    match status {
        CostStatus::ProviderReported => "provider_reported",
        CostStatus::Estimated => "estimated",
        CostStatus::Missing => "missing",
        CostStatus::Mixed => "mixed",
    }
}

fn usage_quality_name(quality: &UsageDataQuality) -> &'static str {
    match quality {
        UsageDataQuality::Parsed => "parsed",
    }
}

fn display_path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn usage_summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UsageSummary> {
    let client: String = row.get(0)?;
    let local_day: Option<String> = row.get(1)?;
    let model_provider: Option<String> = row.get(2)?;
    let model: Option<String> = row.get(3)?;
    let project_path = row.get::<_, Option<String>>(4)?.map(PathBuf::from);
    let session_id: Option<String> = row.get(5)?;
    let tokens = UsageTokenBreakdown {
        input: sum_u64(row, 6)?,
        output: sum_u64(row, 7)?,
        cache_read: sum_u64(row, 8)?,
        cache_write: sum_u64(row, 9)?,
        cache_write_5m: sum_optional_u64(row, 10)?,
        cache_write_1h: sum_optional_u64(row, 11)?,
        reasoning: sum_u64(row, 12)?,
        extra: sum_u64(row, 13)?,
    };
    let normalized_total_tokens = sum_u64(row, 14)?;
    let provider_total_tokens = sum_optional_u64(row, 15)?;
    let cost_sum: f64 = row.get::<_, Option<f64>>(16)?.unwrap_or_default();
    let cost_count = sum_u64(row, 17)?;
    let provider_reported_count = sum_u64(row, 18)?;
    let estimated_count = sum_u64(row, 19)?;
    let missing_count = sum_u64(row, 20)?;
    let event_count = sum_u64(row, 21)?;
    let local_hour: Option<String> = row.get(22)?;
    let estimated_cost_usd = (cost_count > 0).then_some(cost_sum);
    let cost_status =
        aggregate_cost_status(provider_reported_count, estimated_count, missing_count);
    Ok(UsageSummary {
        client,
        local_day,
        local_hour,
        model_provider,
        model,
        top_model: None,
        project_path,
        session_id,
        tokens,
        normalized_total_tokens,
        provider_total_tokens,
        estimated_cost_usd,
        cost_status,
        quality: UsageDataQuality::Parsed,
        event_count,
    })
}

fn scan_watermark_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UsageScanWatermark> {
    let fingerprint_json: String = row.get(7)?;
    let source_fingerprint: UsageSourceFingerprint = serde_json::from_str(&fingerprint_json)
        .map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(err))
        })?;
    let source_path: String = row.get(6)?;
    Ok(UsageScanWatermark {
        source_id: row.get(0)?,
        client: row.get(1)?,
        backend: row.get(2)?,
        backend_version: row.get(3)?,
        parser_schema_version: row.get(4)?,
        source_kind: row.get(5)?,
        source_path: PathBuf::from(source_path),
        source_fingerprint,
        last_offset: row.get(8)?,
        last_record_id: row.get(9)?,
        last_scanned_at_unix: row.get(10)?,
        last_scan_status: row.get(11)?,
        diagnostic_code: row.get(12)?,
    })
}

fn sum_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value = row.get::<_, Option<i64>>(index)?.unwrap_or_default();
    Ok(value.max(0) as u64)
}

fn sum_optional_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Option<u64>> {
    row.get::<_, Option<i64>>(index)
        .map(|value| value.map(|value| value.max(0) as u64))
}

fn aggregate_cost_status(
    provider_reported_count: u64,
    estimated_count: u64,
    missing_count: u64,
) -> CostStatus {
    let kinds = [
        provider_reported_count > 0,
        estimated_count > 0,
        missing_count > 0,
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    match (kinds, provider_reported_count, estimated_count) {
        (0, _, _) => CostStatus::Missing,
        (1, count, _) if count > 0 => CostStatus::ProviderReported,
        (1, _, count) if count > 0 => CostStatus::Estimated,
        (1, _, _) => CostStatus::Missing,
        _ => CostStatus::Mixed,
    }
}

fn db_error(path: &Path, err: rusqlite::Error) -> OpenMuxError {
    OpenMuxError::Message(format!("{}: {err}", path.to_string_lossy()))
}

fn db_error_no_path(err: rusqlite::Error) -> OpenMuxError {
    OpenMuxError::Message(format!("sqlite: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AvailabilityState, UsageEventSource, storage::unix_now_nanos};
    use std::{
        env, fs,
        path::{Path, PathBuf},
        thread,
        time::Duration,
    };

    fn insert_test_account(
        store: &StateStore,
        provider: &str,
        auth_hash: &str,
        imported_at_unix: u64,
    ) -> AccountRecord {
        store
            .upsert_account(UpsertAccount {
                provider: provider.to_string(),
                alias: None,
                provider_subject_kind: None,
                provider_subject_hash: None,
                provider_subject_label: None,
                account_label: None,
                plan_label: None,
                auth_type: None,
                expires_at_unix: None,
                auth_hash: auth_hash.to_string(),
                secret_ref: format!("/tmp/{auth_hash}.json"),
                imported_at_unix,
            })
            .unwrap()
    }

    #[test]
    fn merge_account_into_validates_accounts_and_provider() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-merge-validation-{}",
            unix_now_nanos()
        ));
        let store = StateStore::open(&state_root).unwrap();
        let codex = insert_test_account(&store, "codex", "codex-auth", 1);
        let claude = insert_test_account(&store, "claude", "claude-auth", 1);

        let missing = store
            .merge_account_into(&codex.local_id, "missing")
            .unwrap_err();
        assert!(missing.to_string().contains("missing"));

        let mismatch = store
            .merge_account_into(&codex.local_id, &claude.local_id)
            .unwrap_err();
        assert!(mismatch.to_string().contains("different providers"));
        assert!(
            store
                .account_by_local_id(&codex.local_id)
                .unwrap()
                .is_some()
        );
        assert!(
            store
                .account_by_local_id(&claude.local_id)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn merge_account_into_uses_newer_removed_credentials() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-merge-newer-credentials-{}",
            unix_now_nanos()
        ));
        let store = StateStore::open(&state_root).unwrap();
        let keep = insert_test_account(&store, "codex", "older-keep-auth", 10);
        let remove = insert_test_account(&store, "codex", "newer-remove-auth", 20);
        let keep_number = keep.display_number;

        store
            .merge_account_into(&keep.local_id, &remove.local_id)
            .unwrap();

        let merged = store.account_by_local_id(&keep.local_id).unwrap().unwrap();
        assert_eq!(merged.local_id, keep.local_id);
        assert_eq!(merged.display_number, keep_number);
        assert_eq!(merged.auth_hash, remove.auth_hash);
        assert_eq!(merged.secret_ref, remove.secret_ref);
        assert!(
            store
                .account_by_local_id(&remove.local_id)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn sequential_merges_keep_the_newest_credentials() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-merge-sequence-{}",
            unix_now_nanos()
        ));
        let store = StateStore::open(&state_root).unwrap();
        let keep = insert_test_account(&store, "codex", "auth-1", 1);
        let newest = insert_test_account(&store, "codex", "auth-3", 3);
        let middle = insert_test_account(&store, "codex", "auth-2", 2);

        store
            .merge_account_into(&keep.local_id, &newest.local_id)
            .unwrap();
        store
            .merge_account_into(&keep.local_id, &middle.local_id)
            .unwrap();

        let merged = store.account_by_local_id(&keep.local_id).unwrap().unwrap();
        assert_eq!(merged.auth_hash, newest.auth_hash);
        assert_eq!(merged.secret_ref, newest.secret_ref);
        assert_eq!(merged.imported_at_unix, 3);
    }

    #[test]
    fn merge_account_into_uses_active_removed_credentials_even_when_older() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-merge-active-credentials-{}",
            unix_now_nanos()
        ));
        let store = StateStore::open(&state_root).unwrap();
        let keep = insert_test_account(&store, "codex", "newer-keep-auth", 20);
        let remove = insert_test_account(&store, "codex", "older-active-auth", 10);
        let keep_number = keep.display_number;
        store
            .set_active_account("codex", &remove.local_id, 30)
            .unwrap();

        store
            .merge_account_into(&keep.local_id, &remove.local_id)
            .unwrap();

        let merged = store.account_by_local_id(&keep.local_id).unwrap().unwrap();
        assert_eq!(merged.local_id, keep.local_id);
        assert_eq!(merged.display_number, keep_number);
        assert_eq!(merged.auth_hash, remove.auth_hash);
        assert_eq!(merged.secret_ref, remove.secret_ref);
        assert_eq!(
            store.active_account("codex").unwrap().unwrap().local_id,
            keep.local_id
        );
    }

    #[test]
    fn merge_account_into_preserves_colliding_child_rows() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-merge-children-{}",
            unix_now_nanos()
        ));
        let store = StateStore::open(&state_root).unwrap();
        let keep = insert_test_account(&store, "codex", "keep-auth", 1);
        let remove = insert_test_account(&store, "codex", "remove-auth", 2);

        for local_id in [&keep.local_id, &remove.local_id] {
            store
                .conn
                .execute(
                    "INSERT INTO quota_snapshots (local_id, provider, captured_at_unix, source, snapshot_json) VALUES (?1, 'codex', 10, 'remote_api', '{}')",
                    params![local_id],
                )
                .unwrap();
            store
                .conn
                .execute(
                    "INSERT INTO refresh_attempts (local_id, provider, refresh_kind, trigger, attempted_at_unix, status) VALUES (?1, 'codex', 'manual', 'test', 10, 'success')",
                    params![local_id],
                )
                .unwrap();
        }

        store
            .merge_account_into(&keep.local_id, &remove.local_id)
            .unwrap();

        for table in ["quota_snapshots", "refresh_attempts"] {
            let count: u32 = store
                .conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE local_id = ?1"),
                    params![keep.local_id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 2, "all {table} rows should be preserved");
        }
        assert!(
            store
                .account_by_local_id(&remove.local_id)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn merge_account_into_rolls_back_every_change_on_late_failure() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-merge-rollback-{}",
            unix_now_nanos()
        ));
        let store = StateStore::open(&state_root).unwrap();
        let keep = insert_test_account(&store, "codex", "keep-auth", 1);
        let remove = insert_test_account(&store, "codex", "remove-auth", 2);
        store
            .set_active_account("codex", &remove.local_id, 3)
            .unwrap();
        store
            .conn
            .execute(
                "INSERT INTO quota_snapshots (local_id, provider, captured_at_unix, source, snapshot_json) VALUES (?1, 'codex', 10, 'remote_api', '{}')",
                params![remove.local_id],
            )
            .unwrap();
        store
            .conn
            .execute_batch(
                "CREATE TRIGGER fail_account_merge BEFORE DELETE ON accounts BEGIN SELECT RAISE(ABORT, 'forced merge failure'); END;",
            )
            .unwrap();

        assert!(
            store
                .merge_account_into(&keep.local_id, &remove.local_id)
                .is_err()
        );

        assert!(
            store
                .account_by_local_id(&remove.local_id)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            store.active_account("codex").unwrap().unwrap().local_id,
            remove.local_id
        );
        let remove_rows: u32 = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM quota_snapshots WHERE local_id = ?1",
                params![remove.local_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remove_rows, 1);
    }

    #[test]
    fn update_account_auth_is_scoped_to_provider_and_local_id() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-update-auth-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let account = store
            .upsert_account(UpsertAccount {
                provider: "codex".to_string(),
                alias: None,
                provider_subject_kind: None,
                provider_subject_hash: None,
                provider_subject_label: None,
                account_label: None,
                plan_label: None,
                auth_type: None,
                expires_at_unix: None,
                auth_hash: "old-hash".to_string(),
                secret_ref: "/tmp/old.auth.json".to_string(),
                imported_at_unix: 1,
            })
            .unwrap();

        let updated = store
            .update_account_auth(
                "codex",
                &account.local_id,
                "new-hash",
                "/tmp/new.auth.json",
                2,
            )
            .unwrap();
        assert_eq!(updated.auth_hash, "new-hash");
        assert_eq!(updated.secret_ref, "/tmp/new.auth.json");

        let err = store
            .update_account_auth(
                "claude",
                &account.local_id,
                "wrong-hash",
                "/tmp/wrong.auth.json",
                3,
            )
            .unwrap_err();
        assert!(matches!(err, OpenMuxError::AccountNotFound { .. }));
        let unchanged = store
            .account_by_local_id(&account.local_id)
            .unwrap()
            .unwrap();
        assert_eq!(unchanged.auth_hash, "new-hash");
    }

    #[test]
    fn subject_upsert_deletes_duplicate_auth_hash_account() {
        let state_root =
            env::temp_dir().join(format!("openmux-state-store-subject-{}", unix_now_nanos()));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();

        let first = store
            .upsert_account(UpsertAccount {
                provider: "codex".to_string(),
                alias: Some("primary".to_string()),
                provider_subject_kind: Some("codex_chatgpt_account".to_string()),
                provider_subject_hash: Some("subject-hash".to_string()),
                provider_subject_label: Some("chatgpt_account_id".to_string()),
                account_label: Some("person@example.com".to_string()),
                plan_label: Some("Plus".to_string()),
                auth_type: None,
                expires_at_unix: None,
                auth_hash: "auth-1".to_string(),
                secret_ref: "/tmp/auth-1.json".to_string(),
                imported_at_unix: 1,
            })
            .unwrap();
        let duplicate = store
            .upsert_account(UpsertAccount {
                provider: "codex".to_string(),
                alias: Some("duplicate".to_string()),
                provider_subject_kind: None,
                provider_subject_hash: None,
                provider_subject_label: None,
                account_label: Some("person@example.com".to_string()),
                plan_label: Some("Plus".to_string()),
                auth_type: None,
                expires_at_unix: None,
                auth_hash: "auth-2".to_string(),
                secret_ref: "/tmp/auth-2.json".to_string(),
                imported_at_unix: 2,
            })
            .unwrap();

        let updated = store
            .upsert_account(UpsertAccount {
                provider: "codex".to_string(),
                alias: None,
                provider_subject_kind: Some("codex_chatgpt_account".to_string()),
                provider_subject_hash: Some("subject-hash".to_string()),
                provider_subject_label: Some("chatgpt_account_id".to_string()),
                account_label: Some("person@example.com".to_string()),
                plan_label: Some("Plus".to_string()),
                auth_type: None,
                expires_at_unix: None,
                auth_hash: "auth-2".to_string(),
                secret_ref: "/tmp/auth-2.json".to_string(),
                imported_at_unix: 3,
            })
            .unwrap();

        assert_eq!(updated.local_id, first.local_id);
        assert_eq!(updated.auth_hash, "auth-2");
        assert_eq!(store.list_accounts("codex").unwrap().len(), 1);
        assert!(
            store
                .account_by_local_id(&duplicate.local_id)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn remove_account_compacts_display_numbers_before_next_import() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-reuse-gap-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();

        for number in 1..=3 {
            store
                .upsert_account(UpsertAccount {
                    provider: "codex".to_string(),
                    alias: None,
                    provider_subject_kind: None,
                    provider_subject_hash: None,
                    provider_subject_label: None,
                    account_label: Some(format!("account-{number}")),
                    plan_label: None,
                    auth_type: None,
                    expires_at_unix: None,
                    auth_hash: format!("auth-{number}"),
                    secret_ref: format!("/tmp/auth-{number}.json"),
                    imported_at_unix: number,
                })
                .unwrap();
        }

        let second = store.account_by_selector("codex", "2").unwrap().unwrap();
        store.remove_account(&second.local_id).unwrap();
        assert_eq!(
            store
                .list_accounts("codex")
                .unwrap()
                .into_iter()
                .map(|account| (account.display_number, account.account_label))
                .collect::<Vec<_>>(),
            vec![
                (1, Some("account-1".to_string())),
                (2, Some("account-3".to_string()))
            ]
        );

        let replacement = store
            .upsert_account(UpsertAccount {
                provider: "codex".to_string(),
                alias: None,
                provider_subject_kind: None,
                provider_subject_hash: None,
                provider_subject_label: None,
                account_label: Some("replacement".to_string()),
                plan_label: None,
                auth_type: None,
                expires_at_unix: None,
                auth_hash: "auth-replacement".to_string(),
                secret_ref: "/tmp/auth-replacement.json".to_string(),
                imported_at_unix: 4,
            })
            .unwrap();

        assert_eq!(replacement.display_number, 3);
        assert_eq!(
            store
                .list_accounts("codex")
                .unwrap()
                .into_iter()
                .map(|account| account.display_number)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn migration_deletes_legacy_archived_accounts_before_number_assignment() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-legacy-archived-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();

        for number in 1..=3 {
            store
                .upsert_account(UpsertAccount {
                    provider: "codex".to_string(),
                    alias: None,
                    provider_subject_kind: None,
                    provider_subject_hash: None,
                    provider_subject_label: None,
                    account_label: Some(format!("account-{number}")),
                    plan_label: None,
                    auth_type: None,
                    expires_at_unix: None,
                    auth_hash: format!("auth-{number}"),
                    secret_ref: format!("/tmp/auth-{number}.json"),
                    imported_at_unix: number,
                })
                .unwrap();
        }
        let second = store.account_by_selector("codex", "2").unwrap().unwrap();
        store
            .conn
            .execute(
                "UPDATE accounts SET archived_at_unix = 10 WHERE local_id = ?1",
                params![second.local_id],
            )
            .unwrap();
        drop(store);

        let reopened = StateStore::open(&state_root).unwrap();
        let replacement = reopened
            .upsert_account(UpsertAccount {
                provider: "codex".to_string(),
                alias: None,
                provider_subject_kind: None,
                provider_subject_hash: None,
                provider_subject_label: None,
                account_label: Some("replacement".to_string()),
                plan_label: None,
                auth_type: None,
                expires_at_unix: None,
                auth_hash: "auth-replacement".to_string(),
                secret_ref: "/tmp/auth-replacement.json".to_string(),
                imported_at_unix: 4,
            })
            .unwrap();

        assert_eq!(replacement.display_number, 3);
        assert_eq!(reopened.list_accounts("codex").unwrap().len(), 3);
    }

    #[test]
    fn remove_account_deletes_account_scoped_state() {
        let state_root =
            env::temp_dir().join(format!("openmux-state-store-remove-{}", unix_now_nanos()));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let account = store
            .upsert_account(UpsertAccount {
                provider: "codex".to_string(),
                alias: Some("primary".to_string()),
                provider_subject_kind: Some("codex_chatgpt_account".to_string()),
                provider_subject_hash: Some("subject-hash".to_string()),
                provider_subject_label: Some("chatgpt_account_id".to_string()),
                account_label: Some("person@example.com".to_string()),
                plan_label: Some("Plus".to_string()),
                auth_type: None,
                expires_at_unix: None,
                auth_hash: "auth-1".to_string(),
                secret_ref: "/tmp/auth-1.json".to_string(),
                imported_at_unix: 1,
            })
            .unwrap();
        let snapshot = UsageSnapshot {
            source: UsageSource::RemoteApi,
            refreshed_at_unix: Some(10),
            summary: Availability {
                state: AvailabilityState::Available,
                display: "available".to_string(),
            },
            limits: Vec::new(),
            reset_credits: None,
            diagnostics: Vec::new(),
        };

        store
            .set_active_account("codex", &account.local_id, 2)
            .unwrap();
        store
            .save_quota_snapshot(&account.local_id, "codex", &snapshot)
            .unwrap();
        store
            .record_refresh_attempt(&account.local_id, "codex", "success", None, 11)
            .unwrap();

        store.remove_account(&account.local_id).unwrap();

        assert!(
            store
                .account_by_local_id(&account.local_id)
                .unwrap()
                .is_none()
        );
        assert!(store.active_account("codex").unwrap().is_none());
        assert!(
            store
                .latest_quota_snapshot(&account.local_id)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            store
                .conn
                .query_row(
                    "SELECT COUNT(*) FROM refresh_attempts WHERE local_id = ?1",
                    params![account.local_id],
                    |row| row.get::<_, u32>(0),
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn remove_profile_deletes_profile_and_active_target() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-remove-profile-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let profile = store
            .upsert_profile(UpsertProfile {
                provider: "codex".to_string(),
                name: "work".to_string(),
                label: Some("Work".to_string()),
                profile_kind: "api".to_string(),
                provider_id: Some("openai".to_string()),
                base_url: Some("https://api.openai.com".to_string()),
                model: Some("gpt-5".to_string()),
                auth_type: Some("api_key".to_string()),
                config_hash: "config-1".to_string(),
                secret_ref: "/tmp/profile-work.toml".to_string(),
                imported_at_unix: 1,
            })
            .unwrap();

        store
            .set_active_profile("codex", &profile.local_id, 2)
            .unwrap();
        store.remove_profile(&profile.local_id).unwrap();

        assert!(
            store
                .profile_by_local_id(&profile.local_id)
                .unwrap()
                .is_none()
        );
        assert!(store.active_profile("codex").unwrap().is_none());
        assert!(store.list_profiles("codex").unwrap().is_empty());
    }

    #[test]
    fn account_and_profile_are_single_active_target_per_provider() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-independent-targets-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let account = insert_test_account(&store, "codex", "auth-1", 1);
        let profile = store
            .upsert_profile(UpsertProfile {
                provider: "codex".to_string(),
                name: "gateway".to_string(),
                label: None,
                profile_kind: "api".to_string(),
                provider_id: Some("openmux-gateway".to_string()),
                base_url: Some("https://gateway.example/v1".to_string()),
                model: Some("gpt-5".to_string()),
                auth_type: Some("api_key".to_string()),
                config_hash: "config-1".to_string(),
                secret_ref: "/tmp/gateway.config.toml".to_string(),
                imported_at_unix: 1,
            })
            .unwrap();

        store
            .set_active_account_preserving_profile("codex", &account.local_id, 2)
            .unwrap();
        assert!(store.active_account("codex").unwrap().is_some());
        assert!(store.active_profile("codex").unwrap().is_none());

        store
            .set_active_profile_preserving_account("codex", &profile.local_id, 3)
            .unwrap();

        assert!(store.active_account("codex").unwrap().is_none());
        assert_eq!(
            store.active_profile("codex").unwrap().unwrap().local_id,
            profile.local_id
        );

        store
            .set_active_account_preserving_profile("codex", &account.local_id, 4)
            .unwrap();

        assert_eq!(
            store.active_account("codex").unwrap().unwrap().local_id,
            account.local_id
        );
        assert!(store.active_profile("codex").unwrap().is_none());
    }

    #[test]
    fn migration_removes_duplicate_active_targets() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-active-target-migration-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let account = insert_test_account(&store, "codex", "auth-1", 1);
        let profile = store
            .upsert_profile(UpsertProfile {
                provider: "codex".to_string(),
                name: "gateway".to_string(),
                label: None,
                profile_kind: "api".to_string(),
                provider_id: Some("openmux-gateway".to_string()),
                base_url: Some("https://gateway.example/v1".to_string()),
                model: Some("gpt-5".to_string()),
                auth_type: Some("api_key".to_string()),
                config_hash: "config-1".to_string(),
                secret_ref: "/tmp/gateway.config.toml".to_string(),
                imported_at_unix: 1,
            })
            .unwrap();
        store
            .conn
            .execute(
                "INSERT INTO active_targets (provider, target_kind, local_id, activated_at_unix) VALUES ('codex', 'account', ?1, 10)",
                params![account.local_id],
            )
            .unwrap();
        store
            .conn
            .execute(
                "INSERT INTO active_targets (provider, target_kind, local_id, activated_at_unix) VALUES ('codex', 'profile', ?1, 10)",
                params![profile.local_id],
            )
            .unwrap();
        drop(store);

        let reopened = StateStore::open(&state_root).unwrap();

        assert_eq!(
            reopened.active_account("codex").unwrap().unwrap().local_id,
            account.local_id
        );
        assert!(reopened.active_profile("codex").unwrap().is_none());
    }

    #[test]
    fn usage_event_insert_is_idempotent_and_summarizes_by_client() {
        let state_root =
            env::temp_dir().join(format!("openmux-state-store-usage-{}", unix_now_nanos()));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let event = usage_event("codex", "event-1", CostStatus::Missing, None);

        assert!(store.insert_usage_event(&event, 100).unwrap());
        assert!(!store.insert_usage_event(&event, 101).unwrap());

        let summaries = store.usage_summaries(None, None, None).unwrap();
        assert_eq!(summaries.len(), 1);
        let summary = &summaries[0];
        assert_eq!(summary.client, "codex");
        assert_eq!(summary.event_count, 1);
        assert_eq!(summary.tokens.input, 10);
        assert_eq!(summary.tokens.output, 5);
        assert_eq!(summary.tokens.cache_read, 3);
        assert_eq!(summary.tokens.cache_write, 2);
        assert_eq!(summary.tokens.reasoning, 7);
        assert_eq!(summary.tokens.extra, 1);
        assert_eq!(summary.normalized_total_tokens, 28);
        assert_eq!(summary.provider_total_tokens, Some(99));
        assert_eq!(summary.estimated_cost_usd, None);
        assert_eq!(summary.cost_status, CostStatus::Missing);
        assert_eq!(summary.quality, UsageDataQuality::Parsed);
    }

    #[test]
    fn usage_event_hash_conflict_is_rejected() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-usage-conflict-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let event = usage_event("codex", "same-hash", CostStatus::Estimated, Some(0.25));
        let mut conflicting = usage_event("codex", "same-hash", CostStatus::Estimated, Some(0.25));
        conflicting.tokens.output = 500;

        assert!(store.insert_usage_event(&event, 100).unwrap());
        let err = store.insert_usage_event(&conflicting, 101).unwrap_err();

        assert!(err.to_string().contains("usage event hash conflict"));
        let summaries = store.usage_summaries(Some("codex"), None, None).unwrap();
        assert_eq!(summaries[0].tokens.output, 5);
    }

    #[test]
    fn usage_event_cost_only_change_is_idempotent_without_backfill() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-usage-cost-only-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let missing = usage_event("codex", "same-hash", CostStatus::Missing, None);
        let estimated = usage_event("codex", "same-hash", CostStatus::Estimated, Some(0.25));

        assert!(store.insert_usage_event(&missing, 100).unwrap());
        assert!(!store.insert_usage_event(&estimated, 101).unwrap());

        let summaries = store.usage_summaries(Some("codex"), None, None).unwrap();
        assert_eq!(summaries[0].event_count, 1);
        assert_eq!(summaries[0].estimated_cost_usd, None);
        assert_eq!(summaries[0].cost_status, CostStatus::Missing);
    }

    #[test]
    fn usage_summary_filters_by_time_range() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-usage-range-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let mut older = usage_event("claude", "older", CostStatus::ProviderReported, Some(0.10));
        older.occurred_at_unix = 10;
        let mut newer = usage_event("claude", "newer", CostStatus::ProviderReported, Some(0.20));
        newer.occurred_at_unix = 20;

        store.insert_usage_event(&older, 100).unwrap();
        store.insert_usage_event(&newer, 100).unwrap();

        let summaries = store
            .usage_summaries(Some("claude"), Some(15), Some(25))
            .unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].event_count, 1);
        assert_eq!(summaries[0].estimated_cost_usd, Some(0.20));
        assert_eq!(summaries[0].cost_status, CostStatus::ProviderReported);
    }

    #[test]
    fn usage_summary_filters_and_groups_by_model_provider_project_and_session() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-usage-dimensions-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let mut codex_openai = usage_event("codex", "codex-openai", CostStatus::Missing, None);
        codex_openai.model_provider = Some("openai".to_string());
        codex_openai.model = Some("gpt-5".to_string());
        codex_openai.project_path = Some(PathBuf::from("/tmp/project-a"));
        codex_openai.session_id = Some("session-a".to_string());
        let mut codex_google = usage_event("codex", "codex-google", CostStatus::Missing, None);
        codex_google.model_provider = Some("google".to_string());
        codex_google.model = Some("gemini-2.5-pro".to_string());
        codex_google.project_path = Some(PathBuf::from("/tmp/project-b"));
        codex_google.session_id = Some("session-b".to_string());

        store.insert_usage_event(&codex_openai, 100).unwrap();
        store.insert_usage_event(&codex_google, 100).unwrap();

        let summaries = store
            .usage_summaries_by(UsageSummaryQuery {
                client: Some("codex".to_string()),
                model_provider: Some("openai".to_string()),
                project_path: Some(PathBuf::from("/tmp/project-a")),
                group_by_model_provider: true,
                group_by_model: true,
                group_by_project: true,
                group_by_session: true,
                ..UsageSummaryQuery::default()
            })
            .unwrap();

        assert_eq!(summaries.len(), 1);
        let summary = &summaries[0];
        assert_eq!(summary.client, "codex");
        assert_eq!(summary.model_provider.as_deref(), Some("openai"));
        assert_eq!(summary.model.as_deref(), Some("gpt-5"));
        assert_eq!(
            summary.project_path.as_deref(),
            Some(Path::new("/tmp/project-a"))
        );
        assert_eq!(summary.session_id.as_deref(), Some("session-a"));
        assert_eq!(summary.event_count, 1);
    }

    #[test]
    fn usage_summary_groups_by_local_day_and_keeps_unknown_model_tokens() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-usage-day-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let mut first = usage_event("codex", "day-one", CostStatus::Missing, None);
        first.occurred_at_unix = 1_775_174_401;
        first.model = None;
        let mut second = usage_event("codex", "day-two", CostStatus::Missing, None);
        second.occurred_at_unix = 1_777_680_001;
        second.model = Some("gpt-5".to_string());

        store.insert_usage_event(&first, 100).unwrap();
        store.insert_usage_event(&second, 100).unwrap();

        let by_day = store
            .usage_summaries_by(UsageSummaryQuery {
                client: Some("codex".to_string()),
                group_by_local_day: true,
                local_day_offset_seconds: 0,
                ..UsageSummaryQuery::default()
            })
            .unwrap();
        let by_model = store
            .usage_summaries_by(UsageSummaryQuery {
                client: Some("codex".to_string()),
                group_by_model: true,
                ..UsageSummaryQuery::default()
            })
            .unwrap();

        assert_eq!(by_day.len(), 2);
        assert_eq!(by_day[0].local_day.as_deref(), Some("2026-04-03"));
        assert_eq!(by_day[1].local_day.as_deref(), Some("2026-05-02"));
        assert_eq!(
            by_day
                .iter()
                .map(|summary| summary.normalized_total_tokens)
                .sum::<u64>(),
            56
        );
        assert!(by_model.iter().any(|summary| summary.model.is_none()));
        assert_eq!(
            by_model
                .iter()
                .map(|summary| summary.normalized_total_tokens)
                .sum::<u64>(),
            56
        );
    }

    #[test]
    fn usage_summary_groups_by_local_hour() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-usage-hour-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        // 2026-04-03 12:00:01 and 12:59:59 fall in the same hour; 13:00:01 is the next.
        let mut first = usage_event("codex", "hour-a", CostStatus::Missing, None);
        first.occurred_at_unix = 1_775_174_401;
        let mut second = usage_event("codex", "hour-a2", CostStatus::Missing, None);
        second.occurred_at_unix = first.occurred_at_unix + 3_500;
        let mut third = usage_event("codex", "hour-b", CostStatus::Missing, None);
        third.occurred_at_unix = first.occurred_at_unix + 3_700;

        store.insert_usage_event(&first, 10).unwrap();
        store.insert_usage_event(&second, 20).unwrap();
        store.insert_usage_event(&third, 30).unwrap();

        let by_hour = store
            .usage_summaries_by(UsageSummaryQuery {
                client: Some("codex".to_string()),
                group_by_local_hour: true,
                local_day_offset_seconds: 0,
                ..UsageSummaryQuery::default()
            })
            .unwrap();

        assert_eq!(by_hour.len(), 2);
        assert_eq!(by_hour[0].local_hour.as_deref(), Some("2026-04-03T00"));
        assert_eq!(by_hour[0].event_count, 2);
        assert_eq!(by_hour[1].local_hour.as_deref(), Some("2026-04-03T01"));
        assert_eq!(by_hour[1].event_count, 1);
        assert_eq!(
            by_hour[0].normalized_total_tokens,
            2 * by_hour[1].normalized_total_tokens
        );
    }

    #[test]
    fn scan_watermark_round_trips_fingerprint_and_staleness_metadata() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-watermark-{}",
            unix_now_nanos()
        ));
        let source_root = state_root.join("sources");
        fs::create_dir_all(&source_root).unwrap();
        let source_path = source_root.join("session.jsonl");
        let sidecar_path = source_root.join("session.meta.json");
        fs::write(&source_path, b"{\"tokens\":1}\n").unwrap();
        fs::write(&sidecar_path, b"{\"agent\":\"codex\"}\n").unwrap();

        let store = StateStore::open(&state_root).unwrap();
        let fingerprint = UsageSourceFingerprint::from_path_with_related(
            &source_path,
            [(".meta.json", sidecar_path)],
            7,
            "tokscale-commit",
        )
        .unwrap();
        let watermark = UsageScanWatermark {
            source_id: "codex:session".to_string(),
            client: "codex".to_string(),
            backend: "tokscale".to_string(),
            backend_version: "tokscale-commit".to_string(),
            parser_schema_version: 7,
            source_kind: "jsonl".to_string(),
            source_path: source_path.clone(),
            source_fingerprint: fingerprint.clone(),
            last_offset: Some(128),
            last_record_id: Some("line-3".to_string()),
            last_scanned_at_unix: 1234,
            last_scan_status: "success".to_string(),
            diagnostic_code: None,
        };

        store.update_scan_watermark(&watermark).unwrap();
        let loaded = store
            .scan_watermark("codex:session")
            .unwrap()
            .expect("watermark should exist");

        assert_eq!(loaded.source_id, watermark.source_id);
        assert_eq!(loaded.client, "codex");
        assert_eq!(loaded.source_path, source_path);
        assert_eq!(loaded.source_fingerprint, fingerprint);
        assert_eq!(loaded.last_offset, Some(128));
        assert_eq!(loaded.last_record_id.as_deref(), Some("line-3"));
        assert!(!loaded.is_stale_for("tokscale-commit", 7));
        assert!(loaded.is_stale_for("tokscale-next", 7));
    }

    #[test]
    fn ingest_usage_events_commits_events_and_watermark_in_one_transaction() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-usage-ingest-{}",
            unix_now_nanos()
        ));
        let source_root = state_root.join("sources");
        fs::create_dir_all(&source_root).unwrap();
        let source_path = source_root.join("session.jsonl");
        fs::write(&source_path, b"{\"tokens\":1}\n").unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let fingerprint = UsageSourceFingerprint::from_path(&source_path, 1, "test-backend")
            .expect("fingerprint");
        let watermark = UsageScanWatermark {
            source_id: "codex:session".to_string(),
            client: "codex".to_string(),
            backend: "test-backend".to_string(),
            backend_version: "0.0.0".to_string(),
            parser_schema_version: 1,
            source_kind: "jsonl".to_string(),
            source_path,
            source_fingerprint: fingerprint,
            last_offset: Some(42),
            last_record_id: None,
            last_scanned_at_unix: 200,
            last_scan_status: "success".to_string(),
            diagnostic_code: None,
        };
        let event = usage_event("codex", "ingested-event", CostStatus::Missing, None);

        let inserted = store
            .ingest_usage_events(&[event], Some(&watermark), 123)
            .unwrap();

        assert_eq!(inserted, 1);
        assert_eq!(
            store
                .scan_watermark("codex:session")
                .unwrap()
                .expect("watermark")
                .last_offset,
            Some(42)
        );
        assert_eq!(
            store
                .usage_summaries(Some("codex"), None, None)
                .unwrap()
                .first()
                .expect("summary")
                .event_count,
            1
        );
    }

    #[test]
    fn ingest_usage_events_rolls_back_batch_and_watermark_on_event_conflict() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-usage-ingest-rollback-{}",
            unix_now_nanos()
        ));
        let source_root = state_root.join("sources");
        fs::create_dir_all(&source_root).unwrap();
        let source_path = source_root.join("session.jsonl");
        fs::write(&source_path, b"{\"tokens\":1}\n").unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let existing = usage_event("codex", "same-hash", CostStatus::Missing, None);
        let new_event = usage_event("codex", "new-in-batch", CostStatus::Missing, None);
        let mut conflicting = usage_event("codex", "same-hash", CostStatus::Missing, None);
        conflicting.tokens.output = 500;
        let watermark = usage_watermark("codex:session", &source_path);

        store.insert_usage_event(&existing, 100).unwrap();
        let err = store
            .ingest_usage_events(&[new_event, conflicting], Some(&watermark), 123)
            .unwrap_err();

        assert!(err.to_string().contains("usage event hash conflict"));
        assert!(store.scan_watermark("codex:session").unwrap().is_none());
        let summaries = store.usage_summaries(Some("codex"), None, None).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].event_count, 1);
        assert_eq!(summaries[0].tokens.output, 5);
    }

    #[test]
    fn usage_insert_waits_for_short_sqlite_write_lock() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-usage-busy-timeout-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let db_path = state_root.join("omx-state.sqlite");
        let lock_conn = Connection::open(&db_path).unwrap();
        lock_conn.execute_batch("BEGIN IMMEDIATE;").unwrap();
        drop(store);

        let insert_state_root = state_root.clone();
        let handle = thread::spawn(move || {
            let store = StateStore::open(&insert_state_root).unwrap();
            let event = usage_event("codex", "busy-wait-event", CostStatus::Missing, None);
            store.insert_usage_event(&event, 100).unwrap()
        });

        thread::sleep(Duration::from_millis(100));
        assert!(
            !handle.is_finished(),
            "usage insert finished while another connection held a write lock"
        );
        lock_conn.execute_batch("COMMIT;").unwrap();

        assert!(handle.join().unwrap());
        let store = StateStore::open(&state_root).unwrap();
        let summaries = store.usage_summaries(Some("codex"), None, None).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].event_count, 1);
    }

    #[test]
    fn usage_events_do_not_store_raw_sensitive_source_text() {
        let state_root = env::temp_dir().join(format!(
            "openmux-state-store-usage-privacy-{}",
            unix_now_nanos()
        ));
        let source_root = state_root.join("sources");
        fs::create_dir_all(&source_root).unwrap();
        let source_path = source_root.join("session.jsonl");
        let raw_log_line = concat!(
            r#"{"prompt":"raw prompt secret","response":"raw response secret","#,
            r#""access_token":"access-token-secret","api_key":"sk-live-secret"}"#,
            "\n"
        );
        fs::write(&source_path, raw_log_line.as_bytes()).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let fingerprint =
            UsageSourceFingerprint::from_path(&source_path, 1, "test-backend").unwrap();
        let mut event = usage_event("codex", "privacy-event", CostStatus::Missing, None);
        event.source.path = Some(source_path);
        event.source.fingerprint_json = Some(fingerprint.to_json().unwrap());
        event.source.record_hash = Some("hashed-record-only".to_string());

        store.insert_usage_event(&event, 100).unwrap();
        store
            .conn
            .execute_batch("PRAGMA wal_checkpoint(FULL);")
            .unwrap();

        let sqlite_text = sqlite_files_text(&state_root);
        for forbidden in [
            raw_log_line.trim(),
            "raw prompt secret",
            "raw response secret",
            "access-token-secret",
            "sk-live-secret",
        ] {
            assert!(
                !sqlite_text.contains(forbidden),
                "SQLite usage store leaked sensitive source text: {forbidden}"
            );
        }
    }

    fn usage_watermark(source_id: &str, source_path: &Path) -> UsageScanWatermark {
        let fingerprint =
            UsageSourceFingerprint::from_path(source_path, 1, "test-backend").unwrap();
        UsageScanWatermark {
            source_id: source_id.to_string(),
            client: "codex".to_string(),
            backend: "test-backend".to_string(),
            backend_version: "0.0.0".to_string(),
            parser_schema_version: 1,
            source_kind: "jsonl".to_string(),
            source_path: source_path.to_path_buf(),
            source_fingerprint: fingerprint,
            last_offset: Some(42),
            last_record_id: None,
            last_scanned_at_unix: 200,
            last_scan_status: "success".to_string(),
            diagnostic_code: None,
        }
    }

    fn sqlite_files_text(state_root: &Path) -> String {
        ["omx-state.sqlite", "omx-state.sqlite-wal"]
            .into_iter()
            .map(|file_name| state_root.join(file_name))
            .filter(|path| path.exists())
            .map(|path| String::from_utf8_lossy(&fs::read(path).unwrap()).into_owned())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn usage_event(
        client: &str,
        event_hash: &str,
        cost_status: CostStatus,
        cost: Option<f64>,
    ) -> UsageEvent {
        UsageEvent {
            client: client.to_string(),
            model_provider: Some("openai".to_string()),
            model: Some("gpt-5".to_string()),
            session_id: Some("session-1".to_string()),
            request_id: Some(format!("request-{event_hash}")),
            project_path: Some(PathBuf::from("/tmp/project")),
            occurred_at_unix: 42,
            tokens: UsageTokenBreakdown {
                input: 10,
                output: 5,
                cache_read: 3,
                cache_write: 2,
                cache_write_5m: Some(2),
                cache_write_1h: None,
                reasoning: 7,
                extra: 1,
            },
            provider_total_tokens: Some(99),
            estimated_cost_usd: cost,
            cost_status,
            source: UsageEventSource {
                kind: "jsonl".to_string(),
                path: Some(PathBuf::from("/tmp/session.jsonl")),
                fingerprint_json: Some(r#"{"size":123}"#.to_string()),
                offset: Some(12),
                record_id: None,
                record_hash: Some(format!("line-hash-{event_hash}")),
                backend: "test-backend".to_string(),
                backend_version: "0.0.0".to_string(),
                parser_schema_version: 1,
            },
            quality: UsageDataQuality::Parsed,
            event_hash: event_hash.to_string(),
        }
    }
}
