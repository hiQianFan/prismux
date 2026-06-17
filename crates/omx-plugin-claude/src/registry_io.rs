use super::{AccountRegistry, Registry, StoredAccount, StoredProfile};
use omx_core::{OpenMuxError, Result, storage::display_path};
use std::path::Path;

pub(super) fn encode_registry(registry: &Registry) -> String {
    let mut output = String::new();
    output.push_str(&format!("schema_version\t{}\n", registry.schema_version));
    if let Some(number) = registry.active_profile_number {
        output.push_str(&format!("active_profile_number\t{number}\n"));
    }
    output.push_str(&format!(
        "next_profile_number\t{}\n",
        registry.next_profile_number
    ));
    for profile in &registry.profiles {
        output.push_str(&format!(
            "profile\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            profile.number,
            escape_field(&profile.name),
            escape_field(&profile.auth_type),
            escape_field(profile.base_url.as_deref().unwrap_or("")),
            escape_field(profile.model.as_deref().unwrap_or("")),
            escape_field(&profile.secret_hash),
            escape_field(&profile.snapshot_path),
            profile.imported_at_unix,
            profile
                .last_activated_at_unix
                .map(|value| value.to_string())
                .unwrap_or_default()
        ));
    }
    output
}

pub(super) fn parse_registry(path: &Path, text: &str) -> Result<Registry> {
    let mut registry = Registry::default();
    let mut saw_schema = false;
    for (line_number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        match fields.as_slice() {
            ["schema_version", value] => {
                registry.schema_version = parse_number(path, line_number, value, "schema version")?;
                saw_schema = true;
            }
            ["active_profile_number", value] => {
                registry.active_profile_number = Some(parse_number(
                    path,
                    line_number,
                    value,
                    "active profile number",
                )?);
            }
            ["next_profile_number", value] => {
                registry.next_profile_number =
                    parse_number(path, line_number, value, "next profile number")?;
            }
            [
                "profile",
                number,
                name,
                auth_type,
                base_url,
                model,
                secret_hash,
                snapshot_path,
                imported_at,
                last_activated_at,
            ] => {
                let base_url = unescape_field(base_url)?;
                let model = unescape_field(model)?;
                registry.profiles.push(StoredProfile {
                    number: parse_number(path, line_number, number, "profile number")?,
                    name: unescape_field(name)?,
                    auth_type: unescape_field(auth_type)?,
                    base_url: (!base_url.is_empty()).then_some(base_url),
                    model: (!model.is_empty()).then_some(model),
                    secret_hash: unescape_field(secret_hash)?,
                    snapshot_path: unescape_field(snapshot_path)?,
                    imported_at_unix: parse_number(
                        path,
                        line_number,
                        imported_at,
                        "import timestamp",
                    )?,
                    last_activated_at_unix: if last_activated_at.is_empty() {
                        None
                    } else {
                        Some(parse_number(
                            path,
                            line_number,
                            last_activated_at,
                            "activation timestamp",
                        )?)
                    },
                });
            }
            _ => {
                return Err(OpenMuxError::Message(format!(
                    "{}:{}: unrecognized registry line",
                    display_path(path),
                    line_number + 1
                )));
            }
        }
    }
    if !saw_schema {
        return Err(OpenMuxError::Message(format!(
            "{}: missing schema_version",
            display_path(path)
        )));
    }
    Ok(registry)
}

pub(super) fn encode_account_registry(registry: &AccountRegistry) -> String {
    let mut output = String::new();
    output.push_str(&format!("schema_version\t{}\n", registry.schema_version));
    if let Some(number) = registry.active_account_number {
        output.push_str(&format!("active_account_number\t{number}\n"));
    }
    output.push_str(&format!(
        "next_account_number\t{}\n",
        registry.next_account_number
    ));
    for account in &registry.accounts {
        output.push_str(&format!(
            "account\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            account.number,
            escape_field(&account.name),
            escape_field(account.email.as_deref().unwrap_or("")),
            escape_field(account.account_uuid_hash.as_deref().unwrap_or("")),
            escape_field(account.organization_uuid_hash.as_deref().unwrap_or("")),
            escape_field(account.scopes.as_deref().unwrap_or("")),
            account.expires_at_unix,
            escape_field(&account.refresh_token_hash),
            escape_field(&account.snapshot_hash),
            escape_field(&account.snapshot_path),
            escape_field(&account.oauth_account_path),
            account.partial_metadata,
            account.imported_at_unix,
            account
                .last_activated_at_unix
                .map(|value| value.to_string())
                .unwrap_or_default()
        ));
    }
    output
}

pub(super) fn parse_account_registry(path: &Path, text: &str) -> Result<AccountRegistry> {
    let mut registry = AccountRegistry::default();
    let mut saw_schema = false;
    for (line_number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        match fields.as_slice() {
            ["schema_version", value] => {
                registry.schema_version = parse_number(path, line_number, value, "schema version")?;
                saw_schema = true;
            }
            ["active_account_number", value] => {
                registry.active_account_number = Some(parse_number(
                    path,
                    line_number,
                    value,
                    "active account number",
                )?);
            }
            ["next_account_number", value] => {
                registry.next_account_number =
                    parse_number(path, line_number, value, "next account number")?;
            }
            [
                "account",
                number,
                name,
                email,
                account_uuid_hash,
                organization_uuid_hash,
                scopes,
                expires_at_unix,
                refresh_token_hash,
                snapshot_hash,
                snapshot_path,
                oauth_account_path,
                partial_metadata,
                imported_at_unix,
                last_activated_at_unix,
            ] => {
                let email = unescape_field(email)?;
                let account_uuid_hash = unescape_field(account_uuid_hash)?;
                let organization_uuid_hash = unescape_field(organization_uuid_hash)?;
                let scopes = unescape_field(scopes)?;
                registry.accounts.push(StoredAccount {
                    number: parse_number(path, line_number, number, "account number")?,
                    name: unescape_field(name)?,
                    email: (!email.is_empty()).then_some(email),
                    account_uuid_hash: (!account_uuid_hash.is_empty()).then_some(account_uuid_hash),
                    organization_uuid_hash: (!organization_uuid_hash.is_empty())
                        .then_some(organization_uuid_hash),
                    scopes: (!scopes.is_empty()).then_some(scopes),
                    expires_at_unix: parse_number(
                        path,
                        line_number,
                        expires_at_unix,
                        "expires timestamp",
                    )?,
                    refresh_token_hash: unescape_field(refresh_token_hash)?,
                    snapshot_hash: unescape_field(snapshot_hash)?,
                    snapshot_path: unescape_field(snapshot_path)?,
                    oauth_account_path: unescape_field(oauth_account_path)?,
                    partial_metadata: parse_number(
                        path,
                        line_number,
                        partial_metadata,
                        "partial metadata flag",
                    )?,
                    imported_at_unix: parse_number(
                        path,
                        line_number,
                        imported_at_unix,
                        "import timestamp",
                    )?,
                    last_activated_at_unix: if last_activated_at_unix.is_empty() {
                        None
                    } else {
                        Some(parse_number(
                            path,
                            line_number,
                            last_activated_at_unix,
                            "activation timestamp",
                        )?)
                    },
                });
            }
            _ => {
                return Err(OpenMuxError::Message(format!(
                    "{}:{}: unrecognized account registry line",
                    display_path(path),
                    line_number + 1
                )));
            }
        }
    }
    if !saw_schema {
        return Err(OpenMuxError::Message(format!(
            "{}: missing schema_version",
            display_path(path)
        )));
    }
    Ok(registry)
}

fn parse_number<T>(path: &Path, line_number: usize, value: &str, label: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value.parse().map_err(|err| {
        OpenMuxError::Message(format!(
            "{}:{}: invalid {label}: {err}",
            display_path(path),
            line_number + 1
        ))
    })
}

fn escape_field(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\t', "%09")
        .replace('\n', "%0A")
}

fn unescape_field(value: &str) -> Result<String> {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            output.push(ch);
            continue;
        }
        let code = [
            chars.next().ok_or_else(|| {
                OpenMuxError::Message("invalid percent escape in registry".into())
            })?,
            chars.next().ok_or_else(|| {
                OpenMuxError::Message("invalid percent escape in registry".into())
            })?,
        ];
        match code {
            ['2', '5'] => output.push('%'),
            ['0', '9'] => output.push('\t'),
            ['0', 'A'] => output.push('\n'),
            _ => {
                return Err(OpenMuxError::Message(
                    "invalid percent escape in registry".into(),
                ));
            }
        }
    }
    Ok(output)
}
