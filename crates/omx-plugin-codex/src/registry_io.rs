use super::{Registry, StoredAccount, validate_alias_option};
use omx_core::{OpenMuxError, Result, storage::display_path};
use std::path::Path;

pub(super) fn encode_registry(registry: &Registry) -> String {
    let mut output = String::new();
    output.push_str(&format!("schema_version\t{}\n", registry.schema_version));
    if let Some(number) = registry.active_number {
        output.push_str(&format!("active_number\t{number}\n"));
    }
    if let Some(number) = registry.previous_active_number {
        output.push_str(&format!("previous_active_number\t{number}\n"));
    }
    output.push_str(&format!("next_number\t{}\n", registry.next_number));
    for account in &registry.accounts {
        output.push_str(&format!(
            "account\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            account.number,
            escape_field(account.alias.as_deref().unwrap_or("")),
            escape_field(account.account_label.as_deref().unwrap_or("")),
            escape_field(account.plan_label.as_deref().unwrap_or("")),
            escape_field(&account.auth_hash),
            escape_field(&account.snapshot_path),
            account.imported_at_unix,
            account
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
            ["active_number", value] => {
                registry.active_number =
                    Some(parse_number(path, line_number, value, "active number")?);
            }
            ["previous_active_number", value] => {
                registry.previous_active_number = Some(parse_number(
                    path,
                    line_number,
                    value,
                    "previous active number",
                )?);
            }
            ["next_number", value] => {
                registry.next_number = parse_number(path, line_number, value, "next number")?;
            }
            [
                "account",
                number,
                alias,
                account_label,
                plan_label,
                auth_hash,
                snapshot_path,
                imported_at,
                last_activated_at,
            ] => {
                let imported_at_unix =
                    parse_number(path, line_number, imported_at, "import timestamp")?;
                let last_activated_at_unix = if last_activated_at.is_empty() {
                    None
                } else {
                    Some(parse_number(
                        path,
                        line_number,
                        last_activated_at,
                        "activation timestamp",
                    )?)
                };
                let alias = unescape_field(alias)?;
                let alias = if alias.is_empty() { None } else { Some(alias) };
                validate_alias_option(alias.as_deref())?;
                let account_label = unescape_field(account_label)?;
                let account_label = if account_label.is_empty() {
                    None
                } else {
                    Some(account_label)
                };
                let plan_label = unescape_field(plan_label)?;
                let plan_label = if plan_label.is_empty() {
                    None
                } else {
                    Some(plan_label)
                };

                registry.accounts.push(StoredAccount {
                    number: parse_number(path, line_number, number, "account number")?,
                    alias,
                    account_label,
                    plan_label,
                    auth_hash: unescape_field(auth_hash)?,
                    snapshot_path: unescape_field(snapshot_path)?,
                    imported_at_unix,
                    last_activated_at_unix,
                });
            }
            [
                "account",
                number,
                alias,
                auth_hash,
                snapshot_path,
                imported_at,
                last_activated_at,
            ] => {
                let imported_at_unix =
                    parse_number(path, line_number, imported_at, "import timestamp")?;
                let last_activated_at_unix = if last_activated_at.is_empty() {
                    None
                } else {
                    Some(parse_number(
                        path,
                        line_number,
                        last_activated_at,
                        "activation timestamp",
                    )?)
                };
                let alias = unescape_field(alias)?;
                let alias = if alias.is_empty() { None } else { Some(alias) };
                validate_alias_option(alias.as_deref())?;

                registry.accounts.push(StoredAccount {
                    number: parse_number(path, line_number, number, "account number")?,
                    alias,
                    account_label: None,
                    plan_label: None,
                    auth_hash: unescape_field(auth_hash)?,
                    snapshot_path: unescape_field(snapshot_path)?,
                    imported_at_unix,
                    last_activated_at_unix,
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
