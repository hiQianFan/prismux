use anstream::println;
use anstyle::{AnsiColor, Style};
use anyhow::{Context, Result, bail};
use chrono::{Local, TimeZone};
use clap::{Parser, Subcommand};
use comfy_table::{
    Attribute, Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS,
    presets::UTF8_FULL_CONDENSED,
};
use inquire::Select;
use omx_core::{
    AccountRef, AccountStatus, ConfigProfile, ImportConfigOptions, LoginOptions, PlatformPlugin,
    SaveOptions, UsageLimit, UsageSnapshot, UseReport,
};
use omx_plugin_codex::CodexPlugin;
use std::{
    fmt,
    io::{self, IsTerminal, Read},
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

#[derive(Debug, Parser)]
#[command(name = "omx", version, about = "OpenMux CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List platform pools or accounts for one platform.
    List {
        /// Optional platform id, for example: codex.
        platform: Option<String>,
    },
    /// Show the current account for a platform.
    Current {
        /// Optional platform id, for example: codex.
        platform: Option<String>,
    },
    /// Login through a platform's official flow and record the account.
    Login {
        /// Platform id, for example: codex.
        platform: String,
        /// Use the provider's device authorization login mode.
        #[arg(long)]
        device_auth: bool,
        /// Optional local alias to save after login.
        #[arg(long)]
        alias: Option<String>,
        /// Switch to the new account after login succeeds.
        #[arg(long = "use")]
        use_account: bool,
    },
    /// Show status for known platforms.
    Status,
    /// Import external gateway/provider config for a platform.
    Import {
        /// Platform id, for example: codex.
        platform: String,
        /// Optional profile name, for example: apikey-fun.
        #[arg(long)]
        name: Option<String>,
        /// Read config content from a file.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Read config content from the system clipboard when supported.
        #[arg(long)]
        clipboard: bool,
        /// Config content. Put pasted TOML or KEY=VALUE pairs at the end.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        content: Vec<String>,
    },
    /// Save the currently active account as a recovery/advanced path.
    Save {
        /// Platform id, for example: codex.
        platform: String,
        /// Optional local alias.
        #[arg(long)]
        alias: Option<String>,
    },
    /// Switch the active account.
    Use {
        /// Platform id, for example: codex.
        platform: String,
        /// Account number or alias. If omitted, OpenMux will ask you to choose.
        selector: Option<String>,
    },
    /// Set or replace a local alias for an account.
    Alias {
        /// Platform id, for example: codex.
        platform: String,
        /// Account number or existing alias.
        selector: String,
        /// New alias.
        alias: String,
    },
    /// Run platform diagnostics.
    Doctor {
        /// Optional platform id, for example: codex.
        platform: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let plugins: Vec<Box<dyn PlatformPlugin>> = vec![Box::new(CodexPlugin::new())];

    match cli.command {
        Command::List { platform } => {
            if let Some(platform) = platform {
                let plugin = find_plugin(&plugins, &platform)?;
                print_platform_accounts(plugin)?;
            } else {
                print_section("Overview");
                let mut table = view_table();
                table.set_header(vec![
                    header_cell("Platform"),
                    header_cell("Active"),
                    header_cell("Accts"),
                    header_cell("Overall"),
                    header_cell("5h"),
                    header_cell("Status"),
                ]);
                for plugin in &plugins {
                    let accounts = plugin.list_accounts()?;
                    let active = accounts.iter().find(|status| status.active);
                    table.add_row(vec![
                        Cell::new(plugin.name()).add_attribute(Attribute::Bold),
                        active_account_cell(
                            active
                                .map(|status| account_label(&status.account))
                                .unwrap_or_else(|| "-".to_string()),
                        ),
                        Cell::new(accounts.len()),
                        usage_cell(&summarize_cli_availability(&accounts)),
                        usage_cell(&summarize_window_availability(&accounts, 18_000)),
                        pool_status_cell(attention_count(&accounts)),
                    ]);
                }
                println!("{table}");
            }
        }
        Command::Current { platform } => {
            if let Some(platform) = platform {
                let plugin = find_plugin(&plugins, &platform)?;
                match plugin.current()? {
                    Some(status) => {
                        println!("{}", active_account_label(plugin.name(), &status.account))
                    }
                    None => println!("{}", muted("no active account")),
                }
            } else {
                print_section("Current");
                let mut table = view_table();
                table.set_header(vec![header_cell("Platform"), header_cell("Active")]);
                for plugin in &plugins {
                    match plugin.current()? {
                        Some(status) => {
                            table.add_row(vec![
                                Cell::new(plugin.name()).add_attribute(Attribute::Bold),
                                Cell::new(account_label(&status.account)).fg(Color::Green),
                            ]);
                        }
                        None => {
                            table.add_row(vec![
                                Cell::new(plugin.name()).add_attribute(Attribute::Bold),
                                muted_cell("-"),
                            ]);
                        }
                    }
                }
                println!("{table}");
            }
        }
        Command::Login {
            platform,
            device_auth,
            alias,
            use_account,
        } => {
            let plugin = find_plugin(&plugins, &platform)?;
            let account = plugin.login(LoginOptions {
                device_auth,
                alias,
                activate: use_account,
            })?;
            print_success(format!(
                "Imported {} account {}",
                plugin.name(),
                account_label(&account)
            ));
            if use_account {
                print_success(format!(
                    "Using {} account {}",
                    plugin.name(),
                    account_label(&account)
                ));
                print_hint(format!(
                    "Restart {} if it is already running.",
                    plugin.name()
                ));
            }
        }
        Command::Status => {
            print_section("Platform status");
            let mut table = view_table();
            table.set_header(vec![
                header_cell("Platform"),
                header_cell("ID"),
                header_cell("Config"),
                header_cell("Auth"),
            ]);
            for plugin in &plugins {
                let install = plugin.detect()?;
                table.add_row(vec![
                    Cell::new(install.platform.name).add_attribute(Attribute::Bold),
                    muted_cell(install.platform.id),
                    path_cell(install.config_path.as_deref()),
                    path_cell(install.auth_path.as_deref()),
                ]);
            }
            println!("{table}");
        }
        Command::Import {
            platform,
            name,
            file,
            clipboard,
            content,
        } => {
            let plugin = find_plugin(&plugins, &platform)?;
            let content = read_import_content(file.as_deref(), clipboard, content)?;
            let imported = plugin.import_config(ImportConfigOptions { name, content })?;
            print_success(format!(
                "Imported {} gateway profile `{}`",
                imported.platform.name, imported.profile_name
            ));
            print_hint(format!("Profile config: {}", imported.config_path));
            print_hint("Your main Codex config.toml was not changed.");
            print_hint(format!(
                "Run: {} --profile {}",
                plugin.id(),
                imported.profile_name
            ));
            print_hint(format!("List profiles: omx list {}", plugin.id()));
        }
        Command::Save { platform, alias } => {
            let plugin = find_plugin(&plugins, &platform)?;
            let account = plugin.save_current(SaveOptions { alias })?;
            print_success(format!(
                "Saved {} account {}",
                plugin.name(),
                account_label(&account)
            ));
        }
        Command::Use { platform, selector } => {
            let plugin = find_plugin(&plugins, &platform)?;
            let selector = match selector {
                Some(selector) => selector,
                None => choose_account(plugin)?,
            };
            match plugin.use_target(&selector)? {
                UseReport::Account(report) => {
                    print_success(format!(
                        "Using {} account {}",
                        plugin.name(),
                        account_label(&report.current)
                    ));
                    print_hint(format!(
                        "Restart {} if it is already running.",
                        plugin.name()
                    ));
                }
                UseReport::Config(report) => {
                    print_success(format!(
                        "Using {} profile `{}`",
                        report.platform.name, report.profile.name
                    ));
                    print_hint(format!("Default config: {}", report.config_path));
                    if let Some(backup_path) = report.backup_path {
                        print_hint(format!("Backup: {backup_path}"));
                    }
                    print_hint(format!("Run: {}", plugin.id()));
                    print_hint(format!(
                        "Restart {} App if it is already running.",
                        plugin.name()
                    ));
                }
            }
        }
        Command::Alias {
            platform,
            selector,
            alias,
        } => {
            let plugin = find_plugin(&plugins, &platform)?;
            let account = plugin.set_alias(&selector, &alias)?;
            print_success(format!(
                "Updated {} account {}",
                plugin.name(),
                account_label(&account)
            ));
        }
        Command::Doctor { platform } => {
            if let Some(platform) = platform {
                let plugin = find_plugin(&plugins, &platform)?;
                print_doctor(plugin)?;
            } else {
                for plugin in &plugins {
                    print_doctor(plugin.as_ref())?;
                }
            }
        }
    }

    Ok(())
}

fn find_plugin<'a>(
    plugins: &'a [Box<dyn PlatformPlugin>],
    platform: &str,
) -> Result<&'a dyn PlatformPlugin> {
    plugins
        .iter()
        .map(|plugin| plugin.as_ref())
        .find(|plugin| plugin.id() == platform)
        .with_context(|| format!("unknown platform `{platform}`"))
}

fn read_import_content(
    file: Option<&Path>,
    clipboard: bool,
    content: Vec<String>,
) -> Result<String> {
    if let Some(path) = file {
        return std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()));
    }

    if clipboard {
        return read_clipboard_content();
    }

    if !content.is_empty() {
        if content.len() == 1 {
            let value = &content[0];
            if let Some(path) = value.strip_prefix('@') {
                return std::fs::read_to_string(path)
                    .with_context(|| format!("failed to read config file {path}"));
            }
            let path = Path::new(value);
            if path.exists() && path.is_file() {
                return std::fs::read_to_string(path)
                    .with_context(|| format!("failed to read config file {}", path.display()));
            }
        }
        return Ok(content.join(" "));
    }

    if !io::stdin().is_terminal() {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        if !input.trim().is_empty() {
            return Ok(input);
        }
    }

    bail!(
        "missing config content. Paste TOML/KV at the end, pass --file <path>, use @<path>, --clipboard, or pipe through stdin"
    );
}

fn read_clipboard_content() -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        let output = ProcessCommand::new("pbpaste")
            .output()
            .context("failed to run pbpaste")?;
        if !output.status.success() {
            bail!("pbpaste did not complete successfully");
        }
        let content = String::from_utf8(output.stdout).context("clipboard content is not UTF-8")?;
        if content.trim().is_empty() {
            bail!("clipboard did not contain config content");
        }
        Ok(content)
    }

    #[cfg(not(target_os = "macos"))]
    {
        bail!("--clipboard is currently only supported on macOS");
    }
}

fn print_platform_accounts(plugin: &dyn PlatformPlugin) -> Result<()> {
    let accounts = plugin.list_accounts()?;
    let profiles = plugin.list_configs()?;
    let active = accounts.iter().find(|status| status.active);
    let title = match active {
        Some(status) => format!(
            "{} accounts: {} total, active {}",
            plugin.name(),
            accounts.len(),
            account_label(&status.account)
        ),
        None => format!(
            "{} accounts: {} total, no active account",
            plugin.name(),
            accounts.len()
        ),
    };
    print_section(title);

    if accounts.is_empty() {
        println!(
            "{}",
            muted(
                "No saved accounts. Run `omx login codex --use` or `omx save codex --alias <name>`."
            )
        );
    } else {
        let mut table = view_table();
        table.set_header(account_table_header());
        for status in &accounts {
            table.add_row(account_table_row(status));
        }
        println!("{table}");
    }

    print_platform_profiles(plugin, &profiles);
    Ok(())
}

fn print_platform_profiles(plugin: &dyn PlatformPlugin, profiles: &[ConfigProfile]) {
    println!();
    print_section(format!("{} profiles: {}", plugin.name(), profiles.len()));
    if profiles.is_empty() {
        println!(
            "{}",
            muted("No imported profiles. Run `omx import codex --file <config.toml>`.")
        );
        return;
    }

    let mut table = view_table();
    table.set_header(vec![
        header_cell("*"),
        header_cell("Name"),
        header_cell("Provider"),
        header_cell("Base URL"),
        header_cell("Model"),
        header_cell("Config"),
    ]);
    for profile in profiles {
        table.add_row(vec![
            active_marker_cell(profile.active),
            Cell::new(&profile.name).add_attribute(Attribute::Bold),
            text_or_empty_cell(profile.provider_id.as_deref()),
            text_or_empty_cell(profile.base_url.as_deref()),
            text_or_empty_cell(profile.model.as_deref()),
            muted_cell(&profile.config_path),
        ]);
    }
    println!("{table}");
}

fn account_table_header() -> Vec<Cell> {
    vec![
        header_cell("*"),
        header_cell("#"),
        header_cell("Alias"),
        header_cell("Account"),
        header_cell("Plan"),
        header_cell("5h"),
        header_cell("Weekly"),
        header_cell("Status"),
    ]
}

fn account_table_row(status: &AccountStatus) -> Vec<Cell> {
    let usage = status.usage.as_ref();

    vec![
        active_marker_cell(status.active),
        Cell::new(status.account.number),
        text_or_empty_cell(status.account.alias.as_deref()),
        text_or_unknown_cell(status.account_label.as_deref()),
        text_or_unknown_cell(status.plan_label.as_deref()),
        usage_badge(&usage_limit_with_reset_display(
            usage.and_then(|usage| find_window_limit(usage, 18_000)),
        )),
        usage_badge(&usage_limit_with_reset_display(
            usage.and_then(|usage| find_window_limit(usage, 604_800)),
        )),
        status_badge(&usage_status_display(usage, &status.availability)),
    ]
}

fn choose_account(plugin: &dyn PlatformPlugin) -> Result<String> {
    let accounts = plugin.list_accounts()?;
    if accounts.is_empty() {
        bail!("no saved accounts for platform `{}`", plugin.id());
    }

    let options: Vec<AccountChoice> = accounts.iter().map(AccountChoice::from_status).collect();
    let selected = Select::new("Select account", options).prompt()?;

    Ok(selected.selector)
}

#[derive(Debug, Clone)]
struct AccountChoice {
    selector: String,
    label: String,
}

impl AccountChoice {
    fn from_status(status: &AccountStatus) -> Self {
        let usage = status.usage.as_ref();
        let active = if status.active { "* " } else { "  " };
        let alias = status.account.alias.as_deref().unwrap_or("-");
        let account = status.account_label.as_deref().unwrap_or("unknown");
        let plan = status.plan_label.as_deref().unwrap_or("unknown");
        let five_hour = usage_limit_with_reset_display(
            usage.and_then(|usage| find_window_limit(usage, 18_000)),
        );
        let weekly = usage_limit_with_reset_display(
            usage.and_then(|usage| find_window_limit(usage, 604_800)),
        );

        Self {
            selector: status.account.number.to_string(),
            label: format!(
                "{active}#{} {alias} · {account} · {plan} · 5h {five_hour} · weekly {weekly}",
                status.account.number
            ),
        }
    }
}

impl fmt::Display for AccountChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.label.fmt(f)
    }
}

fn summarize_cli_availability(accounts: &[AccountStatus]) -> String {
    let usage_percentages: Vec<f64> = accounts
        .iter()
        .filter_map(|status| status.usage.as_ref())
        .flat_map(|usage| usage.limits.iter())
        .filter_map(|limit| limit.remaining_percent_x100)
        .map(|percent| percent as f64 / 100.0)
        .collect();
    if !usage_percentages.is_empty() {
        return average_percent(usage_percentages.into_iter());
    }

    let summary_percentages: Vec<f64> = accounts
        .iter()
        .filter_map(|status| parse_display_percent(&status.availability.display))
        .collect();

    average_percent(summary_percentages.into_iter())
}

fn summarize_window_availability(accounts: &[AccountStatus], window_seconds: u64) -> String {
    let percentages: Vec<u32> = accounts
        .iter()
        .filter_map(|status| {
            status
                .usage
                .as_ref()
                .and_then(|usage| find_window_limit(usage, window_seconds))
                .and_then(|limit| limit.remaining_percent_x100)
        })
        .collect();

    average_percent(
        percentages
            .into_iter()
            .map(|percent| percent as f64 / 100.0),
    )
}

fn attention_count(accounts: &[AccountStatus]) -> usize {
    accounts
        .iter()
        .filter(|status| {
            matches!(
                status.availability.state,
                omx_core::AvailabilityState::Limited | omx_core::AvailabilityState::Exhausted
            )
        })
        .count()
}

fn find_window_limit(usage: &UsageSnapshot, window_seconds: u64) -> Option<&UsageLimit> {
    usage
        .limits
        .iter()
        .find(|limit| limit.window_seconds == Some(window_seconds))
}

fn usage_limit_with_reset_display(limit: Option<&UsageLimit>) -> String {
    let Some(limit) = limit else {
        return "-".to_string();
    };

    let percent = limit
        .remaining_percent_x100
        .map(format_percent_x100)
        .unwrap_or_else(|| "-".to_string());

    match limit.reset_at_unix {
        Some(reset_at_unix) if percent != "-" => {
            format!("{percent} ({})", reset_time_display(reset_at_unix))
        }
        _ => percent,
    }
}

fn usage_status_display(
    usage: Option<&UsageSnapshot>,
    availability: &omx_core::Availability,
) -> String {
    let Some(usage) = usage else {
        return availability_state_display(&availability.state).to_string();
    };
    usage
        .diagnostics
        .first()
        .map(|diagnostic| diagnostic.code.clone())
        .unwrap_or_else(|| availability_state_display(&availability.state).to_string())
}

fn availability_state_display(state: &omx_core::AvailabilityState) -> &'static str {
    match state {
        omx_core::AvailabilityState::Unknown => "unknown",
        omx_core::AvailabilityState::Available => "-",
        omx_core::AvailabilityState::Limited => "low",
        omx_core::AvailabilityState::Exhausted => "limited",
    }
}

fn reset_time_display(timestamp: i64) -> String {
    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .map(|time| time.format("%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn average_percent(values: impl Iterator<Item = f64>) -> String {
    let mut total = 0.0;
    let mut count = 0_u32;
    for value in values {
        total += value;
        count += 1;
    }
    if count == 0 {
        return "-".to_string();
    }
    format_percent(total / f64::from(count))
}

fn format_percent_x100(percent_x100: u32) -> String {
    format_percent(percent_x100.min(10_000) as f64 / 100.0)
}

fn format_percent(percent: f64) -> String {
    if (percent.fract()).abs() < f64::EPSILON {
        format!("{}%", percent as i64)
    } else {
        format!("{percent:.1}%")
    }
}

fn parse_display_percent(display: &str) -> Option<f64> {
    display
        .split_whitespace()
        .next()
        .unwrap_or(display)
        .strip_suffix('%')?
        .parse()
        .ok()
}

fn print_doctor(plugin: &dyn PlatformPlugin) -> Result<()> {
    let report = plugin.doctor()?;
    print_section(report.platform);
    let mut table = view_table();
    table.set_header(vec![
        header_cell("Status"),
        header_cell("Check"),
        header_cell("Message"),
    ]);
    for check in report.checks {
        table.add_row(vec![
            if check.ok {
                Cell::new("ok").fg(Color::Green)
            } else {
                Cell::new("fail").fg(Color::Red)
            },
            Cell::new(check.name).add_attribute(Attribute::Bold),
            muted_cell(check.message),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn account_label(account: &AccountRef) -> String {
    match &account.alias {
        Some(alias) => format!("#{} {alias}", account.number),
        None => format!("#{}", account.number),
    }
}

fn active_account_label(platform: &str, account: &AccountRef) -> String {
    format!("{:<10} {}", platform, green(account_label(account)))
}

fn view_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

fn header_cell(value: impl ToString) -> Cell {
    Cell::new(value)
        .fg(Color::DarkGrey)
        .add_attribute(Attribute::Bold)
}

fn muted_cell(value: impl ToString) -> Cell {
    Cell::new(value).fg(Color::DarkGrey)
}

fn path_cell(path: Option<&str>) -> Cell {
    match path {
        Some(path) => Cell::new(path),
        None => muted_cell("not detected yet"),
    }
}

fn text_or_empty_cell(value: Option<&str>) -> Cell {
    match value {
        Some(value) if !value.is_empty() => Cell::new(value),
        _ => muted_cell("-"),
    }
}

fn text_or_unknown_cell(value: Option<&str>) -> Cell {
    match value {
        Some(value) if !value.is_empty() => Cell::new(value),
        _ => muted_cell("unknown"),
    }
}

fn active_marker_cell(active: bool) -> Cell {
    if active {
        Cell::new("*")
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold)
    } else {
        muted_cell("-")
    }
}

fn active_account_cell(value: String) -> Cell {
    if value == "-" {
        muted_cell(value)
    } else {
        Cell::new(value)
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold)
    }
}

fn pool_status_cell(attention_count: usize) -> Cell {
    if attention_count == 0 {
        muted_cell("ok")
    } else {
        Cell::new(format!("{attention_count} limited")).fg(Color::Yellow)
    }
}

fn usage_badge(value: &str) -> Cell {
    if value == "-" || value == "unknown" {
        return muted_cell(value);
    }

    let Some(percent) = parse_display_percent(value) else {
        return Cell::new(value);
    };

    if percent <= 10.0 {
        Cell::new(value).fg(Color::Red)
    } else if percent <= 25.0 {
        Cell::new(value).fg(Color::Yellow)
    } else {
        Cell::new(value).fg(Color::Green)
    }
}

fn usage_cell(value: &str) -> Cell {
    usage_badge(value)
}

fn status_badge(value: &str) -> Cell {
    match value {
        "-" => muted_cell("-"),
        "unknown" => muted_cell("unknown"),
        "low" => Cell::new("low").fg(Color::Yellow),
        "limited" => Cell::new("limited").fg(Color::Red),
        value => Cell::new(value).fg(Color::Yellow),
    }
}

fn print_section(title: impl AsRef<str>) {
    println!("{}", bold(title.as_ref()));
}

fn print_success(message: impl AsRef<str>) {
    println!("{} {}", green("ok"), message.as_ref());
}

fn print_hint(message: impl AsRef<str>) {
    println!("{} {}", muted("hint:"), message.as_ref());
}

fn bold(value: impl AsRef<str>) -> String {
    paint(Style::new().bold(), value)
}

fn muted(value: impl AsRef<str>) -> String {
    paint(Style::new().dimmed(), value)
}

fn green(value: impl AsRef<str>) -> String {
    paint(AnsiColor::Green.on_default().bold(), value)
}

fn paint(style: Style, value: impl AsRef<str>) -> String {
    format!("{style}{}{style:#}", value.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_core::{Availability, AvailabilityState, UsageLimitKind, UsageLimitScope, UsageSource};

    #[test]
    fn usage_limit_display_includes_reset_without_year() {
        let limit = UsageLimit {
            id: "five-hour".to_string(),
            label: "5h".to_string(),
            scope: UsageLimitScope::Account,
            kind: UsageLimitKind::RollingWindow,
            window_seconds: Some(18_000),
            used_percent_x100: None,
            remaining_percent_x100: Some(6_600),
            reset_at_unix: Some(1_785_000_000),
            exhausted: None,
            raw_provider_key: None,
        };

        let display = usage_limit_with_reset_display(Some(&limit));

        assert!(display.starts_with("66% ("));
        assert!(display.ends_with(')'));
        assert!(!display.contains("2026"));
        assert!(!display.contains("1785"));
    }

    #[test]
    fn account_choice_includes_usage_reset_time() {
        let status = AccountStatus {
            account: AccountRef {
                platform: "codex".to_string(),
                number: 2,
                alias: Some("work".to_string()),
            },
            active: true,
            account_label: Some("team@example.com".to_string()),
            plan_label: Some("Pro".to_string()),
            availability: Availability {
                state: AvailabilityState::Available,
                display: "66% remaining".to_string(),
            },
            usage: Some(UsageSnapshot {
                source: UsageSource::RemoteApi,
                refreshed_at_unix: None,
                summary: Availability {
                    state: AvailabilityState::Available,
                    display: "66% remaining".to_string(),
                },
                limits: vec![
                    UsageLimit {
                        id: "five-hour".to_string(),
                        label: "5h".to_string(),
                        scope: UsageLimitScope::Account,
                        kind: UsageLimitKind::RollingWindow,
                        window_seconds: Some(18_000),
                        used_percent_x100: None,
                        remaining_percent_x100: Some(6_600),
                        reset_at_unix: Some(1_785_000_000),
                        exhausted: None,
                        raw_provider_key: None,
                    },
                    UsageLimit {
                        id: "weekly".to_string(),
                        label: "weekly".to_string(),
                        scope: UsageLimitScope::Account,
                        kind: UsageLimitKind::RollingWindow,
                        window_seconds: Some(604_800),
                        used_percent_x100: None,
                        remaining_percent_x100: Some(8_800),
                        reset_at_unix: Some(1_785_086_400),
                        exhausted: None,
                        raw_provider_key: None,
                    },
                ],
                diagnostics: Vec::new(),
            }),
        };

        let choice = AccountChoice::from_status(&status);

        assert_eq!(choice.selector, "2");
        assert!(
            choice
                .label
                .starts_with("* #2 work · team@example.com · Pro")
        );
        assert!(!choice.label.contains("overall"));
        assert!(choice.label.contains("5h 66% ("));
        assert!(choice.label.contains("weekly 88% ("));
        assert!(!choice.label.contains("2026"));
    }

    #[test]
    fn overview_overall_averages_all_usage_limits_not_tightest_summary() {
        let accounts = vec![
            account_status_with_limits(1, 0, 8_000),
            account_status_with_limits(2, 0, 6_000),
        ];

        assert_eq!(summarize_cli_availability(&accounts), "35%");
        assert_eq!(summarize_window_availability(&accounts, 18_000), "0%");
    }

    fn account_status_with_limits(
        number: u32,
        five_hour_percent_x100: u32,
        weekly_percent_x100: u32,
    ) -> AccountStatus {
        AccountStatus {
            account: AccountRef {
                platform: "codex".to_string(),
                number,
                alias: None,
            },
            active: false,
            account_label: None,
            plan_label: None,
            availability: Availability {
                state: AvailabilityState::Exhausted,
                display: "0%".to_string(),
            },
            usage: Some(UsageSnapshot {
                source: UsageSource::RemoteApi,
                refreshed_at_unix: Some(1_785_000_000),
                summary: Availability {
                    state: AvailabilityState::Exhausted,
                    display: "0%".to_string(),
                },
                limits: vec![
                    UsageLimit {
                        id: format!("five-hour-{number}"),
                        label: "5h".to_string(),
                        scope: UsageLimitScope::Account,
                        kind: UsageLimitKind::RollingWindow,
                        window_seconds: Some(18_000),
                        used_percent_x100: None,
                        remaining_percent_x100: Some(five_hour_percent_x100),
                        reset_at_unix: None,
                        exhausted: Some(five_hour_percent_x100 == 0),
                        raw_provider_key: None,
                    },
                    UsageLimit {
                        id: format!("weekly-{number}"),
                        label: "weekly".to_string(),
                        scope: UsageLimitScope::Account,
                        kind: UsageLimitKind::RollingWindow,
                        window_seconds: Some(604_800),
                        used_percent_x100: None,
                        remaining_percent_x100: Some(weekly_percent_x100),
                        reset_at_unix: None,
                        exhausted: Some(weekly_percent_x100 == 0),
                        raw_provider_key: None,
                    },
                ],
                diagnostics: Vec::new(),
            }),
        }
    }
}
