use crate::{
    AccountRef, Availability, ConfigProfile, PlatformInfo, PrismuxError, Result, UsageDiagnostic,
    UsageSnapshot, UsageSource, storage::create_dir_private,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const STATE_STORE_SCHEMA_VERSION: u32 = 2;
const STATE_DB_FILE: &str = "prismux.sqlite";
const LEGACY_PRISMUX_DB_FILE: &str = "prismux-state.sqlite";
const LEGACY_OPENMUX_DB_FILE: &str = "omx-state.sqlite";
const PER_ACCOUNT_HISTORY_LIMIT: i64 = 50;

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

impl StateStore {
    pub fn open(state_root: &Path) -> Result<Self> {
        create_dir_private(state_root)?;
        let path = state_db_path(state_root);
        rename_legacy_prismux_db(state_root, &path)?;
        let conn = Connection::open(&path).map_err(|err| db_error(&path, err))?;
        conn.busy_timeout(std::time::Duration::from_millis(2500))
            .map_err(|err| db_error(&path, err))?;
        let store = Self { conn };
        store.migrate(&path)?;
        store.import_legacy_openmux_state_if_empty(state_root, &path)?;
        store.prune_account_history()?;
        Ok(store)
    }

    fn migrate(&self, path: &Path) -> Result<()> {
        let version: u32 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|err| db_error(path, err))?;
        if version != STATE_STORE_SCHEMA_VERSION {
            self.conn
                .execute_batch(
                    r#"
                    DROP TABLE IF EXISTS usage_events;
                    DROP TABLE IF EXISTS scan_watermarks;
                    DROP TABLE IF EXISTS refresh_attempts;
                    DROP TABLE IF EXISTS quota_snapshots;
                    DROP TABLE IF EXISTS active_targets;
                    DROP TABLE IF EXISTS profiles;
                    DROP TABLE IF EXISTS accounts;
                    "#,
                )
                .map_err(|err| db_error(path, err))?;
        }
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
                    UNIQUE(provider, name),
                    UNIQUE(provider, config_hash)
                );
                CREATE TABLE IF NOT EXISTS active_targets (
                    provider TEXT PRIMARY KEY,
                    target_kind TEXT NOT NULL,
                    local_id TEXT NOT NULL,
                    activated_at_unix INTEGER NOT NULL
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
                    attempted_at_unix INTEGER NOT NULL,
                    status TEXT NOT NULL,
                    error_code TEXT,
                    error_message TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_accounts_provider_active
                    ON accounts(provider, display_number);
                CREATE INDEX IF NOT EXISTS idx_profiles_provider_active
                    ON profiles(provider, display_number, name);
                CREATE INDEX IF NOT EXISTS idx_quota_latest
                    ON quota_snapshots(local_id, captured_at_unix DESC);
                CREATE INDEX IF NOT EXISTS idx_refresh_latest
                    ON refresh_attempts(local_id, attempted_at_unix DESC);
                PRAGMA user_version = 2;
                "#,
            )
            .map_err(|err| db_error(path, err))?;
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

    fn import_legacy_openmux_state_if_empty(&self, state_root: &Path, path: &Path) -> Result<()> {
        if env::var_os("PRISMUX_STATE_ROOT").is_some() || self.target_count()? > 0 {
            return Ok(());
        }
        let Some(parent) = state_root.parent() else {
            return Ok(());
        };
        let legacy = parent.join("openmux").join(LEGACY_OPENMUX_DB_FILE);
        if !legacy.exists() {
            return Ok(());
        }
        self.import_legacy_openmux_state(path, &legacy)
    }

    fn import_legacy_openmux_state(&self, path: &Path, legacy: &Path) -> Result<()> {
        let legacy_path = legacy.to_string_lossy();
        self.conn
            .execute(
                "ATTACH DATABASE ?1 AS legacy",
                params![legacy_path.as_ref()],
            )
            .map_err(|err| db_error(path, err))?;
        let result = self.conn.execute_batch(
            r#"
            INSERT OR IGNORE INTO accounts (
                local_id, provider, display_number, alias, provider_subject_kind,
                provider_subject_hash, provider_subject_label, account_label, plan_label,
                auth_type, expires_at_unix, auth_hash, secret_ref, imported_at_unix,
                updated_at_unix, last_activated_at_unix
            )
            SELECT local_id, provider, display_number, alias, provider_subject_kind,
                   provider_subject_hash, provider_subject_label, account_label, plan_label,
                   auth_type, expires_at_unix, auth_hash, secret_ref, imported_at_unix,
                   updated_at_unix, last_activated_at_unix
            FROM legacy.accounts
            WHERE archived_at_unix IS NULL;

            INSERT OR IGNORE INTO profiles (
                local_id, provider, display_number, name, label, profile_kind,
                provider_id, base_url, model, auth_type, config_hash, secret_ref,
                imported_at_unix, updated_at_unix, last_activated_at_unix
            )
            SELECT local_id, provider, display_number, name, label, profile_kind,
                   provider_id, base_url, model, auth_type, config_hash, secret_ref,
                   imported_at_unix, updated_at_unix, last_activated_at_unix
            FROM legacy.profiles
            WHERE archived_at_unix IS NULL;

            INSERT OR IGNORE INTO active_targets (provider, target_kind, local_id, activated_at_unix)
            SELECT provider, target_kind, local_id, activated_at_unix
            FROM legacy.active_targets
            WHERE EXISTS (SELECT 1 FROM accounts WHERE accounts.local_id = legacy.active_targets.local_id)
               OR EXISTS (SELECT 1 FROM profiles WHERE profiles.local_id = legacy.active_targets.local_id)
            ORDER BY activated_at_unix DESC,
                     CASE target_kind WHEN 'account' THEN 0 ELSE 1 END;

            INSERT INTO quota_snapshots (
                local_id, provider, captured_at_unix, source, snapshot_json, diagnostic_json
            )
            SELECT local_id, provider, captured_at_unix, source, snapshot_json, diagnostic_json
            FROM legacy.quota_snapshots
            WHERE EXISTS (SELECT 1 FROM accounts WHERE accounts.local_id = legacy.quota_snapshots.local_id);

            INSERT INTO refresh_attempts (
                local_id, provider, attempted_at_unix, status, error_code, error_message
            )
            SELECT local_id, provider, attempted_at_unix, status, error_code, error_message
            FROM legacy.refresh_attempts
            WHERE EXISTS (SELECT 1 FROM accounts WHERE accounts.local_id = legacy.refresh_attempts.local_id);
            "#,
        );
        let detach = self.conn.execute_batch("DETACH DATABASE legacy");
        result.map_err(|err| db_error(path, err))?;
        detach.map_err(|err| db_error(path, err))?;
        self.prune_account_history()?;
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
                        updated_at_unix = ?11
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
                "{} WHERE provider = ?1 ORDER BY display_number ASC",
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
                &format!("{} WHERE provider = ?1 AND alias = ?2", account_select()),
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
            return Err(PrismuxError::AccountNotFound {
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
            return Err(PrismuxError::AccountNotFound {
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
                "#,
                params![auth_hash, secret_ref, now, local_id, provider],
            )
            .map_err(db_error_no_path)?;
        if updated != 1 {
            return Err(PrismuxError::AccountNotFound {
                platform: provider.to_string(),
                account: local_id.to_string(),
            });
        }
        self.account_by_local_id(local_id)?
            .ok_or_else(|| PrismuxError::AccountNotFound {
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
            account_details(keep_local_id)?.ok_or_else(|| PrismuxError::AccountNotFound {
                platform: "unknown".to_string(),
                account: keep_local_id.to_string(),
            })?;
        let remove =
            account_details(remove_local_id)?.ok_or_else(|| PrismuxError::AccountNotFound {
                platform: keep.0.clone(),
                account: remove_local_id.to_string(),
            })?;
        if keep.0 != remove.0 {
            return Err(PrismuxError::Message(format!(
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
            .ok_or_else(|| PrismuxError::AccountNotFound {
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
                        imported_at_unix = ?9, updated_at_unix = ?9
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
                WHERE provider = ?1
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
            .map_err(|err| PrismuxError::Message(format!("encode quota snapshot: {err}")))?;
        let diagnostic_json = serde_json::to_string(&usage.diagnostics)
            .map_err(|err| PrismuxError::Message(format!("encode quota diagnostics: {err}")))?;
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
        self.prune_table_for_account(
            "quota_snapshots",
            local_id,
            "captured_at_unix DESC, id DESC",
        )?;
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
                    local_id, provider, attempted_at_unix, status, error_code, error_message
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
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
        self.prune_table_for_account(
            "refresh_attempts",
            local_id,
            "attempted_at_unix DESC, id DESC",
        )?;
        Ok(())
    }

    fn prune_account_history(&self) -> Result<()> {
        self.prune_table("quota_snapshots", "captured_at_unix DESC, id DESC")?;
        self.prune_table("refresh_attempts", "attempted_at_unix DESC, id DESC")
    }

    fn prune_table(&self, table: &str, order: &str) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT DISTINCT local_id FROM {table}"))
            .map_err(db_error_no_path)?;
        let local_ids = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(db_error_no_path)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db_error_no_path)?;
        drop(stmt);
        for local_id in local_ids {
            self.prune_table_for_account(table, &local_id, order)?;
        }
        Ok(())
    }

    fn prune_table_for_account(&self, table: &str, local_id: &str, order: &str) -> Result<()> {
        self.conn
            .execute(
                &format!(
                    r#"
                    DELETE FROM {table}
                    WHERE id IN (
                        SELECT id
                        FROM (
                            SELECT id,
                                   ROW_NUMBER() OVER (ORDER BY {order}) AS rank
                            FROM {table}
                            WHERE local_id = ?1
                        )
                        WHERE rank > ?2
                    )
                    "#
                ),
                params![local_id, PER_ACCOUNT_HISTORY_LIMIT],
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

    fn target_count(&self) -> Result<u64> {
        self.conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM accounts) + (SELECT COUNT(*) FROM profiles)",
                [],
                |row| row.get(0),
            )
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
                &format!("{} WHERE local_id = ?1", account_select()),
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
                    "{} WHERE provider = ?1 AND display_number = ?2",
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
        let sql = profile_select("local_id = ?1");
        self.conn
            .query_row(&sql, params![local_id], profile_from_row)
            .optional()
            .map_err(db_error_no_path)
    }

    fn profile_by_number(&self, provider: &str, number: u32) -> Result<Option<ProfileRecord>> {
        let sql = profile_select("provider = ?1 AND display_number = ?2");
        self.conn
            .query_row(&sql, params![provider, number], profile_from_row)
            .optional()
            .map_err(db_error_no_path)
    }

    fn profile_by_name(&self, provider: &str, name: &str) -> Result<Option<ProfileRecord>> {
        let sql = profile_select("provider = ?1 AND name = ?2");
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
                    (provider, target_kind, local_id, activated_at_unix)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(provider)
                DO UPDATE SET local_id = excluded.local_id,
                              target_kind = excluded.target_kind,
                              activated_at_unix = excluded.activated_at_unix
                "#,
                params![provider, kind.as_str(), local_id, now],
            )
            .map_err(db_error_no_path)?;
        Ok(())
    }

    fn clear_active_local_id(&self, local_id: &str) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM active_targets WHERE local_id = ?1",
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

fn state_db_path(state_root: &Path) -> PathBuf {
    state_root.join(STATE_DB_FILE)
}

fn rename_legacy_prismux_db(state_root: &Path, path: &Path) -> Result<()> {
    let legacy = state_root.join(LEGACY_PRISMUX_DB_FILE);
    if path.exists() || !legacy.exists() {
        return Ok(());
    }
    fs::rename(&legacy, path).map_err(|err| {
        PrismuxError::Message(format!(
            "{} -> {}: {err}",
            legacy.to_string_lossy(),
            path.to_string_lossy()
        ))
    })
}

fn db_error(path: &Path, err: rusqlite::Error) -> PrismuxError {
    PrismuxError::Message(format!("{}: {err}", path.to_string_lossy()))
}

fn db_error_no_path(err: rusqlite::Error) -> PrismuxError {
    PrismuxError::Message(format!("sqlite: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AvailabilityState, storage::unix_now_nanos};
    use std::{env, fs};

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

    fn table_exists(conn: &Connection, table: &str) -> bool {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |_| Ok(()),
        )
        .optional()
        .unwrap()
        .is_some()
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        stmt.query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .any(|name| name.unwrap() == column)
    }

    #[test]
    fn merge_account_into_validates_accounts_and_provider() {
        let state_root = env::temp_dir().join(format!(
            "prismux-state-store-merge-validation-{}",
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
            "prismux-state-store-merge-newer-credentials-{}",
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
            "prismux-state-store-merge-sequence-{}",
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
            "prismux-state-store-merge-active-credentials-{}",
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
            "prismux-state-store-merge-children-{}",
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
                    "INSERT INTO refresh_attempts (local_id, provider, attempted_at_unix, status) VALUES (?1, 'codex', 10, 'success')",
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
            "prismux-state-store-merge-rollback-{}",
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
            "prismux-state-store-update-auth-{}",
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
        assert!(matches!(err, PrismuxError::AccountNotFound { .. }));
        let unchanged = store
            .account_by_local_id(&account.local_id)
            .unwrap()
            .unwrap();
        assert_eq!(unchanged.auth_hash, "new-hash");
    }

    #[test]
    fn subject_upsert_deletes_duplicate_auth_hash_account() {
        let state_root =
            env::temp_dir().join(format!("prismux-state-store-subject-{}", unix_now_nanos()));
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
            "prismux-state-store-reuse-gap-{}",
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
    fn migration_rebuilds_legacy_state_schema() {
        let state_root = env::temp_dir().join(format!(
            "prismux-state-store-legacy-schema-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let path = state_root.join("prismux.sqlite");
        let legacy = Connection::open(&path).unwrap();
        legacy
            .execute_batch(
                r#"
                PRAGMA user_version = 1;
                CREATE TABLE accounts (
                    local_id TEXT PRIMARY KEY,
                    provider TEXT NOT NULL,
                    display_number INTEGER NOT NULL,
                    auth_hash TEXT NOT NULL,
                    secret_ref TEXT NOT NULL,
                    imported_at_unix INTEGER NOT NULL,
                    updated_at_unix INTEGER NOT NULL,
                    archived_at_unix INTEGER
                );
                CREATE TABLE usage_events (id INTEGER PRIMARY KEY AUTOINCREMENT);
                CREATE TABLE scan_watermarks (source_id TEXT PRIMARY KEY);
                INSERT INTO accounts
                    (local_id, provider, display_number, auth_hash, secret_ref, imported_at_unix, updated_at_unix)
                VALUES ('codex_account_1', 'codex', 1, 'auth', '/tmp/auth.json', 1, 1);
                "#,
            )
            .unwrap();
        drop(legacy);

        let reopened = StateStore::open(&state_root).unwrap();

        let version: u32 = reopened
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, STATE_STORE_SCHEMA_VERSION);
        assert!(reopened.list_accounts("codex").unwrap().is_empty());
        assert!(!table_exists(&reopened.conn, "usage_events"));
        assert!(!table_exists(&reopened.conn, "scan_watermarks"));
        assert!(!column_exists(
            &reopened.conn,
            "accounts",
            "archived_at_unix"
        ));
    }

    #[test]
    fn empty_prismux_store_imports_legacy_openmux_targets() {
        let root = env::temp_dir().join(format!(
            "prismux-state-store-openmux-import-{}",
            unix_now_nanos()
        ));
        let state_root = root.join("prismux");
        let legacy_root = root.join("openmux");
        fs::create_dir_all(&legacy_root).unwrap();
        let legacy = Connection::open(legacy_root.join(LEGACY_OPENMUX_DB_FILE)).unwrap();
        legacy
            .execute_batch(
                r#"
                CREATE TABLE accounts (
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
                    archived_at_unix INTEGER
                );
                CREATE TABLE profiles (
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
                    archived_at_unix INTEGER
                );
                CREATE TABLE active_targets (
                    provider TEXT NOT NULL,
                    target_kind TEXT NOT NULL,
                    local_id TEXT NOT NULL,
                    previous_local_id TEXT,
                    activated_at_unix INTEGER NOT NULL,
                    PRIMARY KEY(provider, target_kind)
                );
                CREATE TABLE quota_snapshots (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    local_id TEXT NOT NULL,
                    provider TEXT NOT NULL,
                    captured_at_unix INTEGER NOT NULL,
                    source TEXT NOT NULL,
                    snapshot_json TEXT NOT NULL,
                    diagnostic_json TEXT
                );
                CREATE TABLE refresh_attempts (
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
                CREATE TABLE usage_events (id INTEGER PRIMARY KEY AUTOINCREMENT);
                INSERT INTO accounts (
                    local_id, provider, display_number, alias, provider_subject_kind,
                    provider_subject_hash, provider_subject_label, account_label, plan_label,
                    auth_type, expires_at_unix, auth_hash, secret_ref, imported_at_unix,
                    updated_at_unix, last_activated_at_unix
                )
                VALUES (
                    'codex_account_1', 'codex', 1, 'main', 'kind', 'subject-hash',
                    'subject', 'person@example.com', 'Plus', 'oauth', NULL,
                    'auth-hash', '/tmp/auth.json', 1, 2, 3
                );
                INSERT INTO profiles (
                    local_id, provider, display_number, name, label, profile_kind,
                    provider_id, base_url, model, auth_type, config_hash, secret_ref,
                    imported_at_unix, updated_at_unix, last_activated_at_unix
                )
                VALUES (
                    'codex_profile_1', 'codex', 2, 'gateway', 'Gateway', 'api',
                    'openai', 'https://api.example/v1', 'gpt-5', 'api_key',
                    'config-hash', '/tmp/profile.toml', 4, 5, 6
                );
                INSERT INTO active_targets
                    (provider, target_kind, local_id, activated_at_unix)
                VALUES ('codex', 'account', 'codex_account_1', 10);
                INSERT INTO quota_snapshots
                    (local_id, provider, captured_at_unix, source, snapshot_json)
                VALUES ('codex_account_1', 'codex', 11, 'remote_api', '{}');
                INSERT INTO refresh_attempts
                    (local_id, provider, refresh_kind, trigger, attempted_at_unix, status)
                VALUES ('codex_account_1', 'codex', 'interactive', 'list', 12, 'success');
                INSERT INTO usage_events DEFAULT VALUES;
                "#,
            )
            .unwrap();
        drop(legacy);

        let store = StateStore::open(&state_root).unwrap();

        assert!(state_root.join(STATE_DB_FILE).exists());
        assert_eq!(store.list_accounts("codex").unwrap().len(), 1);
        assert_eq!(store.list_profiles("codex").unwrap().len(), 1);
        assert_eq!(
            store.active_account("codex").unwrap().unwrap().local_id,
            "codex_account_1"
        );
        assert!(!table_exists(&store.conn, "usage_events"));
        assert_eq!(
            store
                .conn
                .query_row("SELECT COUNT(*) FROM quota_snapshots", [], |row| {
                    row.get::<_, u32>(0)
                })
                .unwrap(),
            1
        );
        assert_eq!(
            store
                .conn
                .query_row("SELECT COUNT(*) FROM refresh_attempts", [], |row| {
                    row.get::<_, u32>(0)
                })
                .unwrap(),
            1
        );
    }

    #[test]
    fn remove_account_deletes_account_scoped_state() {
        let state_root =
            env::temp_dir().join(format!("prismux-state-store-remove-{}", unix_now_nanos()));
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
    fn account_history_keeps_recent_rows_only() {
        let state_root = env::temp_dir().join(format!(
            "prismux-state-store-history-limit-{}",
            unix_now_nanos()
        ));
        fs::create_dir_all(&state_root).unwrap();
        let store = StateStore::open(&state_root).unwrap();
        let account = insert_test_account(&store, "codex", "auth-1", 1);

        for index in 0..(PER_ACCOUNT_HISTORY_LIMIT + 5) {
            let snapshot = UsageSnapshot {
                source: UsageSource::RemoteApi,
                refreshed_at_unix: Some(index),
                summary: Availability {
                    state: AvailabilityState::Available,
                    display: "available".to_string(),
                },
                limits: Vec::new(),
                reset_credits: None,
                diagnostics: Vec::new(),
            };
            store
                .save_quota_snapshot(&account.local_id, "codex", &snapshot)
                .unwrap();
            store
                .record_refresh_attempt(&account.local_id, "codex", "success", None, index as u64)
                .unwrap();
        }

        for table in ["quota_snapshots", "refresh_attempts"] {
            let count: i64 = store
                .conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE local_id = ?1"),
                    params![account.local_id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, PER_ACCOUNT_HISTORY_LIMIT);
        }
    }

    #[test]
    fn remove_profile_deletes_profile_and_active_target() {
        let state_root = env::temp_dir().join(format!(
            "prismux-state-store-remove-profile-{}",
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
            "prismux-state-store-independent-targets-{}",
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
                provider_id: Some("prismux-gateway".to_string()),
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
}
