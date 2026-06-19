use crate::{
    Availability, ConfigProfile, OpenMuxError, PlatformInfo, Result, UsageDiagnostic,
    UsageSnapshot, UsageSource, storage::create_dir_private,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    pub account_label: Option<String>,
    pub plan_label: Option<String>,
    pub auth_type: Option<String>,
    pub expires_at_unix: Option<i64>,
    pub auth_hash: String,
    pub secret_ref: String,
    pub imported_at_unix: u64,
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
}

impl StateStore {
    pub fn open(state_root: &Path) -> Result<Self> {
        create_dir_private(state_root)?;
        let path = state_root.join("omx-state.sqlite");
        let conn = Connection::open(&path).map_err(|err| db_error(&path, err))?;
        let store = Self { conn };
        store.migrate(&path)?;
        Ok(store)
    }

    fn migrate(&self, path: &Path) -> Result<()> {
        self.conn
            .execute_batch(
                r#"
                PRAGMA foreign_keys = ON;
                CREATE TABLE IF NOT EXISTS accounts (
                    local_id TEXT PRIMARY KEY,
                    provider TEXT NOT NULL,
                    display_number INTEGER NOT NULL,
                    alias TEXT,
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
                CREATE INDEX IF NOT EXISTS idx_accounts_provider_active
                    ON accounts(provider, archived_at_unix, display_number);
                CREATE INDEX IF NOT EXISTS idx_profiles_provider_active
                    ON profiles(provider, archived_at_unix, display_number, name);
                CREATE INDEX IF NOT EXISTS idx_quota_latest
                    ON quota_snapshots(local_id, captured_at_unix DESC);
                CREATE INDEX IF NOT EXISTS idx_refresh_latest
                    ON refresh_attempts(local_id, attempted_at_unix DESC);
                "#,
            )
            .map_err(|err| db_error(path, err))
    }

    pub fn upsert_account(&self, input: UpsertAccount) -> Result<AccountRecord> {
        if let Some(mut existing) = self.account_by_auth_hash(&input.provider, &input.auth_hash)? {
            self.conn
                .execute(
                    r#"
                    UPDATE accounts
                    SET alias = COALESCE(?1, alias),
                        account_label = COALESCE(?2, account_label),
                        plan_label = COALESCE(?3, plan_label),
                        auth_type = ?4,
                        expires_at_unix = ?5,
                        secret_ref = ?6,
                        imported_at_unix = ?7,
                        updated_at_unix = ?7,
                        archived_at_unix = NULL
                    WHERE local_id = ?8
                    "#,
                    params![
                        input.alias,
                        input.account_label,
                        input.plan_label,
                        input.auth_type,
                        input.expires_at_unix,
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
        let local_id = format!("{}_account_{}", input.provider, display_number);
        self.conn
            .execute(
                r#"
                INSERT INTO accounts (
                    local_id, provider, display_number, alias, account_label, plan_label,
                    auth_type, expires_at_unix, auth_hash, secret_ref, imported_at_unix,
                    updated_at_unix
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
                "#,
                params![
                    local_id,
                    input.provider,
                    display_number,
                    input.alias,
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
            .prepare(
                r#"
                SELECT local_id, provider, display_number, alias, account_label, plan_label,
                       auth_type, expires_at_unix, auth_hash, secret_ref, imported_at_unix,
                       last_activated_at_unix
                FROM accounts
                WHERE provider = ?1 AND archived_at_unix IS NULL
                ORDER BY display_number ASC
                "#,
            )
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
        if let Ok(number) = selector.parse::<u32>() {
            return self.account_by_number(provider, number);
        }
        self.conn
            .query_row(
                r#"
                SELECT local_id, provider, display_number, alias, account_label, plan_label,
                       auth_type, expires_at_unix, auth_hash, secret_ref, imported_at_unix,
                       last_activated_at_unix
                FROM accounts
                WHERE provider = ?1 AND alias = ?2 AND archived_at_unix IS NULL
                "#,
                params![provider, selector],
                account_from_row,
            )
            .optional()
            .map_err(db_error_no_path)
    }

    pub fn set_active_account(&self, provider: &str, local_id: &str, now: u64) -> Result<()> {
        self.set_active_target(provider, TargetKindRecord::Account, local_id, now)?;
        self.conn
            .execute(
                "UPDATE accounts SET last_activated_at_unix = ?1 WHERE local_id = ?2",
                params![now, local_id],
            )
            .map_err(db_error_no_path)?;
        self.clear_active_target(provider, TargetKindRecord::Profile)?;
        Ok(())
    }

    pub fn active_account(&self, provider: &str) -> Result<Option<AccountRecord>> {
        let Some(local_id) = self.active_local_id(provider, TargetKindRecord::Account)? else {
            return Ok(None);
        };
        self.account_by_local_id(&local_id)
    }

    pub fn archive_account(&self, local_id: &str, now: u64) -> Result<()> {
        self.conn
            .execute(
                "UPDATE accounts SET archived_at_unix = ?1, updated_at_unix = ?1 WHERE local_id = ?2",
                params![now, local_id],
            )
            .map_err(db_error_no_path)?;
        self.clear_active_local_id(local_id)
    }

    pub fn set_account_alias(&self, local_id: &str, alias: &str, now: u64) -> Result<()> {
        self.conn
            .execute(
                "UPDATE accounts SET alias = ?1, updated_at_unix = ?2 WHERE local_id = ?3",
                params![alias, now, local_id],
            )
            .map_err(db_error_no_path)?;
        Ok(())
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
        let local_id = format!("{}_profile_{}", input.provider, display_number);
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
        if let Ok(number) = selector.parse::<u32>() {
            return self.profile_by_number(provider, number);
        }
        self.profile_by_name(provider, selector)
    }

    pub fn set_active_profile(&self, provider: &str, local_id: &str, now: u64) -> Result<()> {
        self.set_active_target(provider, TargetKindRecord::Profile, local_id, now)?;
        self.conn
            .execute(
                "UPDATE profiles SET last_activated_at_unix = ?1 WHERE local_id = ?2",
                params![now, local_id],
            )
            .map_err(db_error_no_path)?;
        self.clear_active_target(provider, TargetKindRecord::Account)?;
        Ok(())
    }

    pub fn active_profile(&self, provider: &str) -> Result<Option<ProfileRecord>> {
        let Some(local_id) = self.active_local_id(provider, TargetKindRecord::Profile)? else {
            return Ok(None);
        };
        self.profile_by_local_id(&local_id)
    }

    pub fn archive_profile(&self, local_id: &str, now: u64) -> Result<()> {
        self.conn
            .execute(
                "UPDATE profiles SET archived_at_unix = ?1, updated_at_unix = ?1 WHERE local_id = ?2",
                params![now, local_id],
            )
            .map_err(db_error_no_path)?;
        self.clear_active_local_id(local_id)
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

    fn account_by_auth_hash(
        &self,
        provider: &str,
        auth_hash: &str,
    ) -> Result<Option<AccountRecord>> {
        self.conn
            .query_row(
                r#"
                SELECT local_id, provider, display_number, alias, account_label, plan_label,
                       auth_type, expires_at_unix, auth_hash, secret_ref, imported_at_unix,
                       last_activated_at_unix
                FROM accounts
                WHERE provider = ?1 AND auth_hash = ?2
                "#,
                params![provider, auth_hash],
                account_from_row,
            )
            .optional()
            .map_err(db_error_no_path)
    }

    pub fn account_by_local_id(&self, local_id: &str) -> Result<Option<AccountRecord>> {
        self.conn
            .query_row(
                r#"
                SELECT local_id, provider, display_number, alias, account_label, plan_label,
                       auth_type, expires_at_unix, auth_hash, secret_ref, imported_at_unix,
                       last_activated_at_unix
                FROM accounts
                WHERE local_id = ?1 AND archived_at_unix IS NULL
                "#,
                params![local_id],
                account_from_row,
            )
            .optional()
            .map_err(db_error_no_path)
    }

    fn account_by_number(&self, provider: &str, number: u32) -> Result<Option<AccountRecord>> {
        self.conn
            .query_row(
                r#"
                SELECT local_id, provider, display_number, alias, account_label, plan_label,
                       auth_type, expires_at_unix, auth_hash, secret_ref, imported_at_unix,
                       last_activated_at_unix
                FROM accounts
                WHERE provider = ?1 AND display_number = ?2 AND archived_at_unix IS NULL
                "#,
                params![provider, number],
                account_from_row,
            )
            .optional()
            .map_err(db_error_no_path)
    }

    fn next_account_number(&self, provider: &str) -> Result<u32> {
        let value: Option<u32> = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(display_number), 0) + 1 FROM accounts WHERE provider = ?1",
                params![provider],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error_no_path)?
            .flatten();
        Ok(value.unwrap_or(1))
    }

    fn profile_by_local_id(&self, local_id: &str) -> Result<Option<ProfileRecord>> {
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
        let value: Option<u32> = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(display_number), 0) + 1 FROM profiles WHERE provider = ?1",
                params![provider],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error_no_path)?
            .flatten();
        Ok(value.unwrap_or(1))
    }

    fn set_active_target(
        &self,
        provider: &str,
        kind: TargetKindRecord,
        local_id: &str,
        now: u64,
    ) -> Result<()> {
        let previous = self.active_local_id(provider, kind)?;
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

    fn clear_active_target(&self, provider: &str, kind: TargetKindRecord) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM active_targets WHERE provider = ?1 AND target_kind = ?2",
                params![provider, kind.as_str()],
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
}

impl ProfileRecord {
    pub fn to_config_profile(&self, platform: PlatformInfo, active: bool) -> ConfigProfile {
        ConfigProfile {
            platform,
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

fn account_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccountRecord> {
    Ok(AccountRecord {
        local_id: row.get(0)?,
        provider: row.get(1)?,
        display_number: row.get(2)?,
        alias: row.get(3)?,
        account_label: row.get(4)?,
        plan_label: row.get(5)?,
        auth_type: row.get(6)?,
        expires_at_unix: row.get(7)?,
        auth_hash: row.get(8)?,
        secret_ref: row.get(9)?,
        imported_at_unix: row.get(10)?,
        last_activated_at_unix: row.get(11)?,
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

fn db_error(path: &Path, err: rusqlite::Error) -> OpenMuxError {
    OpenMuxError::Message(format!("{}: {err}", path.to_string_lossy()))
}

fn db_error_no_path(err: rusqlite::Error) -> OpenMuxError {
    OpenMuxError::Message(format!("sqlite: {err}"))
}
