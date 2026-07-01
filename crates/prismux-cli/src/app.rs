use crate::input::{read_import_content, read_optional_import_content};
use anstream::println;
use anstyle::{AnsiColor, Style};
use anyhow::{Context, Result, bail};
use chrono::{Local, TimeZone};
use clap::{Parser, Subcommand};
use comfy_table::{
    Attribute, Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS,
    presets::UTF8_FULL_CONDENSED,
};
use inquire::{Confirm, Select};
use prismux_core::{
    AccountRef, AccountStatus, ConfigProfile, ImportConfigOptions, LoginOptions, PlatformPlugin,
    RemoveReport, ResetCreditOutcome, SaveOptions, StateStore, TargetKind, UsageLimit,
    UsageSnapshot, UseReport,
    storage::{state_root, unix_now},
};
use prismux_plugin_claude::ClaudePlugin;
use prismux_plugin_codex::CodexPlugin;
use std::{
    fmt,
    io::{self, IsTerminal},
    path::PathBuf,
};

#[derive(Debug, Parser)]
#[command(name = "prismux", version, about = "Prismux CLI")]
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
    /// Refresh provider quota snapshots for one platform.
    Refresh {
        /// Platform id, for example: codex.
        platform: String,
        /// Optional account number or alias to refresh one account.
        selector: Option<String>,
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
        /// Account number or alias. If omitted, Prismux will ask you to choose.
        selector: Option<String>,
    },
    /// Remove a managed account or profile from Prismux.
    Remove {
        /// Platform id, for example: codex.
        platform: String,
        /// Account/profile number, alias, or profile name. If omitted, Prismux will ask you to choose.
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
    /// Clear a local alias for an account.
    Unalias {
        /// Platform id, for example: codex.
        platform: String,
        /// Account number or existing alias.
        selector: String,
    },
    /// Consume one Codex reset credit for an account.
    ResetCredit {
        /// Platform id. Currently only codex supports reset credits.
        platform: String,
        /// Account number or alias.
        selector: String,
        /// Confirm without prompting. Required in non-interactive use.
        #[arg(long)]
        yes: bool,
    },
    /// Run platform diagnostics.
    Doctor {
        /// Optional platform id, for example: codex.
        platform: Option<String>,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let plugins: Vec<Box<dyn PlatformPlugin>> =
        vec![Box::new(CodexPlugin::new()), Box::new(ClaudePlugin::new())];

    match cli.command {
        Command::List { platform } => {
            if let Some(platform) = platform {
                print_platform_accounts(find_plugin(&plugins, &platform)?)?;
            } else {
                print_section("Overview");
                let dashboard = prismux_app::dashboard_view(
                    &plugins,
                    prismux_app::DashboardQuery::default(),
                    None,
                )?;
                let mut table = view_table();
                table.set_header(vec![
                    header_cell("Platform"),
                    header_cell("Active"),
                    header_cell("Accts"),
                    header_cell("Profiles"),
                    header_cell("Overall"),
                    header_cell("5h"),
                    header_cell("Status"),
                ]);
                for plugin in visible_plugins(&plugins) {
                    let aggregate = dashboard
                        .aggregate
                        .provider_aggregates
                        .iter()
                        .find(|view| view.provider_id == plugin.id());
                    let active_label = aggregate
                        .and_then(|view| view.active_target.as_ref())
                        .map(|target| target.display_label.clone())
                        .unwrap_or_else(|| "-".to_string());
                    let account_count =
                        aggregate.map(|view| view.account_count).unwrap_or_default();
                    let profile_count =
                        aggregate.map(|view| view.profile_count).unwrap_or_default();
                    let quota = aggregate
                        .and_then(|view| view.quota_health.facts.avg_remaining_percent_x100)
                        .map(percent_x100_display)
                        .unwrap_or_else(|| "-".to_string());
                    let low = aggregate
                        .and_then(|view| view.quota_health.facts.min_remaining_percent_x100)
                        .map(percent_x100_display)
                        .unwrap_or_else(|| "-".to_string());
                    let status = aggregate
                        .map(|view| provider_status_label(&view.status_tone))
                        .unwrap_or("unknown");
                    table.add_row(vec![
                        Cell::new(plugin.name()).add_attribute(Attribute::Bold),
                        active_account_cell(active_label),
                        Cell::new(account_count),
                        Cell::new(profile_count),
                        usage_cell(&quota),
                        usage_cell(&low),
                        provider_status_cell(status),
                    ]);
                }
                println!("{table}");
            }
        }
        Command::Current { platform } => {
            if let Some(platform) = platform {
                let plugin = find_plugin(&plugins, &platform)?;
                if plugin.capabilities().accounts && plugin.capabilities().profiles {
                    print_aggregated_current(plugin)?;
                } else {
                    match prismux_app::active_account_status(plugin)? {
                        Some(status) => {
                            println!("{}", active_account_label(plugin.name(), &status.account))
                        }
                        None => println!("{}", muted("no active account")),
                    }
                }
            } else {
                print_section("Current");
                let mut table = view_table();
                table.set_header(vec![header_cell("Platform"), header_cell("Active")]);
                for plugin in visible_plugins(&plugins) {
                    let current = prismux_app::active_account_status(plugin)?;
                    match current {
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
        Command::Refresh { platform, selector } => {
            let plugin = find_plugin(&plugins, &platform)?;
            match selector {
                Some(selector) => {
                    let account = refresh_selected_account(plugin, &selector)?;
                    print_account_section_from_statuses(plugin.name(), &[account], None);
                }
                None => {
                    let accounts = plugin.refresh_accounts()?;
                    print_account_section_from_statuses(plugin.name(), &accounts, None);
                }
            }
            print_hint(
                "Proxy: set PRISMUX_HTTPS_PROXY, HTTPS_PROXY, or ALL_PROXY before running refresh.",
            );
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
            let display_name = plugin.name();
            print_success(format!(
                "Imported {display_name} account {}",
                account_label(&account)
            ));
            let account_is_active = StateStore::open(&state_root()?)?
                .active_account(plugin.id())?
                .is_some_and(|active| active.local_id == account.local_id);
            if account_is_active {
                print_success(format!(
                    "Using {display_name} account {}",
                    account_label(&account)
                ));
                print_hint(format!(
                    "Restart {} if it is already running.",
                    display_name
                ));
            } else {
                print_hint(login_use_hint(&platform, &account));
            }
        }
        Command::Status => {
            print_section("Platform status");
            let dashboard = prismux_app::dashboard_view(
                &plugins,
                prismux_app::DashboardQuery::default(),
                None,
            )?;
            let mut table = view_table();
            table.set_header(vec![
                header_cell("Platform"),
                header_cell("ID"),
                header_cell("Targets"),
                header_cell("Config"),
                header_cell("Auth"),
            ]);
            for plugin in visible_plugins(&plugins) {
                let install = plugin.detect()?;
                let target_count = dashboard
                    .accounts
                    .accounts
                    .iter()
                    .filter(|account| account.provider == plugin.id())
                    .count()
                    + dashboard
                        .accounts
                        .profiles
                        .iter()
                        .filter(|profile| profile.provider == plugin.id())
                        .count();
                table.add_row(vec![
                    Cell::new(install.platform.name).add_attribute(Attribute::Bold),
                    muted_cell(install.platform.id),
                    Cell::new(target_count),
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
            let requested_content =
                read_optional_import_content(file.as_deref(), clipboard, content)?;
            let plugin = find_plugin(&plugins, &platform)?;
            let (content, imported_kind) = if platform == "claude" {
                match requested_content {
                    Some(content) => (content, ImportKind::Profile),
                    None => (String::new(), ImportKind::Account),
                }
            } else {
                let content = match (plugin.capabilities().account_import, requested_content) {
                    (true, content) => content.unwrap_or_default(),
                    (false, Some(content)) => content,
                    (false, None) => read_import_content(None, false, Vec::new())?,
                };
                let kind = if plugin.capabilities().account_import {
                    ImportKind::Account
                } else {
                    ImportKind::Profile
                };
                (content, kind)
            };
            let imported = plugin.import_config(ImportConfigOptions { name, content })?;
            let display_name = imported.platform.name.as_str();
            if imported_kind == ImportKind::Account {
                print_success(format!(
                    "Imported {display_name} account `{}`",
                    imported.profile_name
                ));
            } else {
                print_success(format!(
                    "Imported {display_name} gateway profile `{}`",
                    imported.profile_name
                ));
            }
            if let Some(number) = imported.number {
                let label = if imported_kind == ImportKind::Account {
                    "Account"
                } else {
                    "Profile"
                };
                print_hint(format!("{label}: #{number} `{}`", imported.profile_name));
            }
            let snapshot_label = if imported_kind == ImportKind::Account {
                "Account snapshot"
            } else {
                "Profile snapshot"
            };
            print_hint(format!("{snapshot_label}: {}", imported.config_path));
            print_hint(format!("List: prismux list {}", plugin.id()));
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
                None => choose_target(plugin, true)?,
            };
            let report = use_resolved_target(plugin, &selector)?;
            match report {
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
        Command::Remove { platform, selector } => {
            let plugin = find_plugin(&plugins, &platform)?;
            let selector = match selector {
                Some(selector) => selector,
                None => choose_target(plugin, true)?,
            };
            let report = remove_resolved_target(plugin, &selector)?;
            match report {
                RemoveReport::Account(report) => {
                    print_success(format!(
                        "Removed {} account {}",
                        plugin.name(),
                        account_label(&report.account)
                    ));
                    if report.was_active {
                        print_hint(
                            "Removed account was active; no replacement account was selected.",
                        );
                    }
                    for path in report.removed_paths {
                        print_hint(format!("Removed: {path}"));
                    }
                }
                RemoveReport::Config(report) => {
                    print_success(format!(
                        "Removed {} profile `{}`",
                        report.profile.platform.name, report.profile.name
                    ));
                    if report.was_active {
                        print_hint(
                            "Removed profile was active; no replacement profile was selected.",
                        );
                    }
                    for path in report.removed_paths {
                        print_hint(format!("Removed: {path}"));
                    }
                }
            }
        }
        Command::Alias {
            platform,
            selector,
            alias,
        } => {
            let plugin = find_plugin(&plugins, &platform)?;
            let target = resolve_target(plugin, &selector)?;
            if target.kind != TargetKind::Account {
                bail!("`{selector}` matched a profile; aliases can only be set on accounts");
            }
            let account = plugin.set_alias(&target.target_id, &alias)?;
            print_success(format!(
                "Updated {} account {}",
                plugin.name(),
                account_label(&account)
            ));
        }
        Command::Unalias { platform, selector } => {
            let plugin = find_plugin(&plugins, &platform)?;
            let target = resolve_target(plugin, &selector)?;
            if target.kind != TargetKind::Account {
                bail!("`{selector}` matched a profile; aliases can only be cleared on accounts");
            }
            let store = StateStore::open(&state_root()?)?;
            let account = store.clear_account_alias_by_selector(
                plugin.id(),
                &target.target_id,
                unix_now(),
            )?;
            print_success(format!(
                "Cleared alias for {} account #{}",
                plugin.name(),
                account.number
            ));
        }
        Command::ResetCredit {
            platform,
            selector,
            yes,
        } => {
            let plugin = find_plugin(&plugins, &platform)?;
            consume_reset_credit_cli(plugin, &selector, yes)?;
        }
        Command::Doctor { platform } => {
            if let Some(platform) = platform {
                print_doctor(find_plugin(&plugins, &platform)?)?;
            } else {
                for plugin in visible_plugins(&plugins) {
                    print_doctor(plugin)?;
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

fn visible_plugins(
    plugins: &[Box<dyn PlatformPlugin>],
) -> impl Iterator<Item = &dyn PlatformPlugin> {
    plugins.iter().map(|plugin| plugin.as_ref())
}

fn refresh_selected_account(plugin: &dyn PlatformPlugin, selector: &str) -> Result<AccountStatus> {
    let target = resolve_target(plugin, selector)?;
    if target.kind != TargetKind::Account {
        bail!("`{selector}` matched a profile; refresh only supports account targets");
    }
    plugin
        .refresh_account(&target.target_id)
        .map_err(Into::into)
}

fn consume_reset_credit_cli(plugin: &dyn PlatformPlugin, selector: &str, yes: bool) -> Result<()> {
    if plugin.id() != "codex" {
        bail!("reset-credit currently supports codex accounts only");
    }
    let target = resolve_target(plugin, selector)?;
    if target.kind != TargetKind::Account {
        bail!("`{selector}` matched a profile; reset-credit only supports account targets");
    }
    confirm_reset_credit(plugin.name(), selector, yes)?;
    let outcome =
        plugin.consume_reset_credit(&target.target_id, &reset_credit_idempotency_key())?;
    print_success(reset_credit_outcome_message(&outcome));
    match plugin.refresh_account(&target.target_id) {
        Ok(account) => print_account_section_from_statuses(plugin.name(), &[account], None),
        Err(err) => print_hint(format!(
            "Reset credit operation completed, but quota refresh failed: {err}"
        )),
    }
    Ok(())
}

fn confirm_reset_credit(platform_name: &str, selector: &str, yes: bool) -> Result<()> {
    require_reset_credit_confirmation_flag(yes, io::stdin().is_terminal())?;
    if yes {
        return Ok(());
    }
    let confirmed = Confirm::new(&format!(
        "Consume 1 {platform_name} reset credit for `{selector}`?"
    ))
    .with_default(false)
    .prompt()?;
    if !confirmed {
        bail!("reset-credit cancelled");
    }
    Ok(())
}

fn require_reset_credit_confirmation_flag(yes: bool, stdin_is_tty: bool) -> Result<()> {
    if !yes && !stdin_is_tty {
        bail!("reset-credit requires --yes when stdin is not a TTY");
    }
    Ok(())
}

fn reset_credit_idempotency_key() -> String {
    format!("prismux-cli-{}-{}", unix_now(), std::process::id())
}

fn reset_credit_outcome_message(outcome: &ResetCreditOutcome) -> String {
    match outcome {
        ResetCreditOutcome::Reset { windows_reset } => {
            format!("Consumed reset credit; reset {windows_reset} usage window(s)")
        }
        ResetCreditOutcome::NothingToReset => {
            "No eligible usage limit to reset; no credit was consumed".to_string()
        }
        ResetCreditOutcome::NoCredit => "No reset credits available".to_string(),
        ResetCreditOutcome::AlreadyRedeemed => {
            "Reset credit request was already redeemed".to_string()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportKind {
    Account,
    Profile,
}

fn use_resolved_target(
    plugin: &dyn PlatformPlugin,
    selector: &str,
) -> Result<prismux_core::UseReport> {
    Ok(prismux_app::use_resolved_target(plugin, selector)?)
}

fn remove_resolved_target(
    plugin: &dyn PlatformPlugin,
    selector: &str,
) -> Result<prismux_core::RemoveReport> {
    Ok(prismux_app::remove_resolved_target(plugin, selector)?)
}

fn resolve_target(
    plugin: &dyn PlatformPlugin,
    selector: &str,
) -> Result<prismux_core::TargetResolution> {
    Ok(prismux_app::resolve_target(plugin, selector)?)
}

fn load_target_catalog(plugin: &dyn PlatformPlugin) -> Result<prismux_core::TargetCatalog> {
    Ok(prismux_app::target_catalog(plugin)?)
}

fn target_choice_from_account(
    status: &AccountStatus,
    display_index: u32,
    use_display_selector: bool,
) -> TargetChoice {
    let usage = status.usage.as_ref();
    let active = if status.active { "* " } else { "  " };
    let alias = status.account.alias.as_deref().unwrap_or("-");
    let account = status.account_label.as_deref().unwrap_or("unknown");
    let plan = status.plan_label.as_deref().unwrap_or("unknown");
    let five_hour =
        usage_limit_with_reset_display(usage.and_then(|usage| find_window_limit(usage, 18_000)));
    let weekly =
        usage_limit_with_reset_display(usage.and_then(|usage| find_window_limit(usage, 604_800)));

    TargetChoice {
        selector: if use_display_selector {
            display_index.to_string()
        } else {
            status.account.number.to_string()
        },
        label: format!(
            "{active}#{display_index} account {alias} · {account} · {plan} · 5h {five_hour} · weekly {weekly}"
        ),
    }
}

fn target_choice_from_profile(
    profile: &ConfigProfile,
    display_index: u32,
    use_display_selector: bool,
) -> TargetChoice {
    let active = if profile.active { "* " } else { "  " };
    let selector = profile
        .number
        .map(|number| number.to_string())
        .unwrap_or_else(|| profile.name.clone());
    let auth = profile.auth_type.as_deref().unwrap_or("-");
    let base_url = profile.base_url.as_deref().unwrap_or("-");
    let model = profile.model.as_deref().unwrap_or("-");

    TargetChoice {
        selector: if use_display_selector {
            display_index.to_string()
        } else {
            selector
        },
        label: format!(
            "{active}#{display_index} profile {} · {auth} · {base_url} · {model}",
            profile.name
        ),
    }
}

fn print_aggregated_current(plugin: &dyn PlatformPlugin) -> Result<()> {
    let account = prismux_app::active_account_status(plugin)?;
    let profile = prismux_app::config_profiles(plugin)?
        .into_iter()
        .find(|profile| profile.active);

    print_section(plugin.name());
    let mut table = view_table();
    table.set_header(vec![header_cell("Kind"), header_cell("Active")]);
    table.add_row(vec![
        Cell::new("Account").add_attribute(Attribute::Bold),
        account
            .map(|status| Cell::new(account_label(&status.account)).fg(Color::Green))
            .unwrap_or_else(|| muted_cell("-")),
    ]);
    table.add_row(vec![
        Cell::new("Profile").add_attribute(Attribute::Bold),
        profile
            .map(|profile| Cell::new(profile.name).fg(Color::Green))
            .unwrap_or_else(|| muted_cell("-")),
    ]);
    println!("{table}");
    Ok(())
}

fn print_platform_accounts(plugin: &dyn PlatformPlugin) -> Result<()> {
    let capabilities = plugin.capabilities();
    if capabilities.accounts && capabilities.profiles {
        let catalog = load_target_catalog(plugin)?;
        print_account_section_from_statuses(plugin.name(), &catalog.accounts, Some(1));
        print_platform_profiles_with_start(
            plugin,
            &catalog.profiles,
            Some(catalog.accounts.len() as u32 + 1),
        );
        if catalog.accounts.is_empty() && catalog.profiles.is_empty() {
            print_hint(format!("Add account: prismux login {}", plugin.id()));
            print_hint(format!(
                "Add profile: prismux import {} --file <path>",
                plugin.id()
            ));
        }
        return Ok(());
    }

    if capabilities.accounts {
        print_account_section(plugin.name(), plugin)?;
    }

    if capabilities.profiles {
        let profiles = prismux_app::config_profiles(plugin)?;
        print_platform_profiles(plugin, &profiles);
    }
    Ok(())
}

fn print_account_section(title_name: &str, plugin: &dyn PlatformPlugin) -> Result<()> {
    let accounts = prismux_app::account_statuses(plugin)?;
    print_account_section_from_statuses(title_name, &accounts, None);
    Ok(())
}

fn print_account_section_from_statuses(
    title_name: &str,
    accounts: &[AccountStatus],
    display_start: Option<u32>,
) {
    let active = accounts.iter().find(|status| status.active);
    let title = match active {
        Some(status) => format!(
            "{} accounts: {} total, active {}",
            title_name,
            accounts.len(),
            account_label(&status.account)
        ),
        None => format!(
            "{} accounts: {} total, no active account",
            title_name,
            accounts.len()
        ),
    };
    print_section(title);

    let mut table = view_table();
    table.set_header(account_table_header());
    for (index, status) in accounts.iter().enumerate() {
        let display_number = display_start
            .map(|start| start + index as u32)
            .unwrap_or(status.account.number);
        table.add_row(account_table_row(status, display_number));
    }
    println!("{table}");
}

fn print_platform_profiles(plugin: &dyn PlatformPlugin, profiles: &[ConfigProfile]) {
    print_platform_profiles_with_start(plugin, profiles, None);
}

fn print_platform_profiles_with_start(
    plugin: &dyn PlatformPlugin,
    profiles: &[ConfigProfile],
    display_start: Option<u32>,
) {
    println!();
    print_section(format!("{} profiles: {}", plugin.name(), profiles.len()));
    if profiles.is_empty() {
        println!(
            "{}",
            muted(format!(
                "No imported profiles. Run `prismux import {} --file <path>`.",
                plugin.id()
            ))
        );
        return;
    }

    let mut table = view_table();
    table.set_header(vec![
        header_cell("*"),
        header_cell("#"),
        header_cell("Name"),
        header_cell("Auth"),
        header_cell("Provider"),
        header_cell("Base URL"),
        header_cell("Model"),
        header_cell("Snapshot"),
    ]);
    for (index, profile) in profiles.iter().enumerate() {
        let display_number = display_start.map(|start| start + index as u32);
        table.add_row(vec![
            active_marker_cell(profile.active),
            display_number
                .or(profile.number)
                .map(Cell::new)
                .unwrap_or_else(|| muted_cell("-")),
            Cell::new(&profile.name).add_attribute(Attribute::Bold),
            text_or_empty_cell(profile.auth_type.as_deref()),
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
        header_cell("Refresh"),
        header_cell("Status"),
    ]
}

fn account_table_row(status: &AccountStatus, display_number: u32) -> Vec<Cell> {
    let usage = status.usage.as_ref();

    vec![
        active_marker_cell(status.active),
        Cell::new(display_number),
        text_or_empty_cell(status.account.alias.as_deref()),
        text_or_unknown_cell(status.account_label.as_deref()),
        text_or_unknown_cell(status.plan_label.as_deref()),
        usage_badge(&usage_limit_with_reset_display(
            usage.and_then(|usage| find_window_limit(usage, 18_000)),
        )),
        usage_badge(&usage_limit_with_reset_display(
            usage.and_then(|usage| find_window_limit(usage, 604_800)),
        )),
        muted_cell(usage_refresh_display(usage)),
        status_badge(&usage_status_display(usage, &status.availability)),
    ]
}

fn choose_target(plugin: &dyn PlatformPlugin, use_display_selector: bool) -> Result<String> {
    let catalog = load_target_catalog(plugin)?;
    if catalog.accounts.is_empty() && catalog.profiles.is_empty() {
        bail!(
            "no saved accounts or profiles for platform `{}`",
            plugin.id()
        );
    }

    let mut options: Vec<TargetChoice> = catalog
        .accounts
        .iter()
        .enumerate()
        .map(|(index, status)| {
            target_choice_from_account(
                status,
                catalog.account_display_index(index),
                use_display_selector,
            )
        })
        .collect();
    options.extend(catalog.profiles.iter().enumerate().map(|(index, profile)| {
        target_choice_from_profile(
            profile,
            catalog.profile_display_index(index),
            use_display_selector,
        )
    }));
    let selected = Select::new("Select account or profile", options).prompt()?;

    Ok(selected.selector)
}

#[derive(Debug, Clone)]
struct TargetChoice {
    selector: String,
    label: String,
}

impl fmt::Display for TargetChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.label.fmt(f)
    }
}

#[cfg(test)]
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

#[cfg(test)]
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
    availability: &prismux_core::Availability,
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

fn usage_refresh_display(usage: Option<&UsageSnapshot>) -> String {
    usage
        .and_then(|usage| usage.refreshed_at_unix)
        .map(reset_time_display)
        .unwrap_or_else(|| "-".to_string())
}

fn availability_state_display(state: &prismux_core::AvailabilityState) -> &'static str {
    match state {
        prismux_core::AvailabilityState::Unknown => "unknown",
        prismux_core::AvailabilityState::Available => "-",
        prismux_core::AvailabilityState::Limited => "low",
        prismux_core::AvailabilityState::Exhausted => "limited",
    }
}

fn reset_time_display(timestamp: i64) -> String {
    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .map(|time| time.format("%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
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

fn login_use_hint(platform: &str, account: &AccountRef) -> String {
    let selector = account
        .alias
        .clone()
        .unwrap_or_else(|| account.number.to_string());
    format!("Account imported but not active. Use it with: prismux use {platform} {selector}")
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

fn percent_x100_display(percent: u32) -> String {
    format!("{}%", percent / 100)
}

fn provider_status_label(tone: &prismux_app::ViewTone) -> &'static str {
    match tone {
        prismux_app::ViewTone::Success => "ok",
        prismux_app::ViewTone::Warning => "limited",
        prismux_app::ViewTone::Danger => "alert",
        prismux_app::ViewTone::Neutral => "-",
    }
}

fn provider_status_cell(value: &str) -> Cell {
    match value {
        "ok" | "-" => muted_cell(value),
        "limited" => Cell::new(value).fg(Color::Yellow),
        "alert" => Cell::new(value).fg(Color::Red),
        value => muted_cell(value),
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
    use clap::CommandFactory;
    use prismux_core::{
        Availability, AvailabilityState, UsageLimitKind, UsageLimitScope, UsageSource,
    };

    #[test]
    fn login_use_hint_prefers_alias_and_falls_back_to_number() {
        let mut account = AccountRef {
            platform: "codex".to_string(),
            local_id: "codex-account-2".to_string(),
            number: 2,
            alias: Some("work".to_string()),
        };
        assert_eq!(
            login_use_hint("codex", &account),
            "Account imported but not active. Use it with: prismux use codex work"
        );

        account.alias = None;
        assert_eq!(
            login_use_hint("codex", &account),
            "Account imported but not active. Use it with: prismux use codex 2"
        );
    }

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
                local_id: "codex-account-2".to_string(),
                number: 2,
                alias: Some("work".to_string()),
            },
            active: true,
            account_label: Some("team@example.com".to_string()),
            plan_label: Some("Pro".to_string()),
            auth_type: None,
            expires_at_unix: None,
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
                reset_credits: None,
                diagnostics: Vec::new(),
            }),
        };

        let choice = target_choice_from_account(&status, 1, false);

        assert_eq!(choice.selector, "2");
        assert!(
            choice
                .label
                .starts_with("* #1 account work · team@example.com · Pro")
        );
        assert!(!choice.label.contains("overall"));
        assert!(choice.label.contains("5h 66% ("));
        assert!(choice.label.contains("weekly 88% ("));
        assert!(!choice.label.contains("2026"));
    }

    #[test]
    fn target_choice_can_return_display_selector_for_aggregated_picker() {
        let profile = config_profile(1, "gateway");

        let choice = target_choice_from_profile(&profile, 3, true);

        assert_eq!(choice.selector, "3");
        assert!(choice.label.starts_with("  #3 profile gateway"));
    }

    #[test]
    fn account_table_header_omits_auth_and_expires() {
        let header = account_table_header()
            .into_iter()
            .map(|cell| cell.content().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            header,
            vec![
                "*", "#", "Alias", "Account", "Plan", "5h", "Weekly", "Refresh", "Status"
            ]
        );
    }

    #[test]
    fn help_includes_remove_command() {
        let mut help = Vec::new();
        Cli::command().write_long_help(&mut help).unwrap();
        let help = String::from_utf8(help).unwrap();

        assert!(help.contains("remove"));
        assert!(help.contains("Remove a managed account or profile"));
        assert!(help.contains("unalias"));
        assert!(help.contains("Clear a local alias for an account"));
        assert!(help.contains("reset-credit"));
        assert!(help.contains("Consume one Codex reset credit"));
    }

    #[test]
    fn refresh_selected_account_uses_resolved_account_target() {
        let plugin = FakePlugin {
            id: "codex",
            name: "Codex",
            accounts: vec![account_status_with_alias_for("codex", 2, Some("work"))],
            profiles: Vec::new(),
            reset_outcome: None,
        };

        let refreshed = refresh_selected_account(&plugin, "work").unwrap();

        assert_eq!(refreshed.account.local_id, "codex-account-2");
    }

    #[test]
    fn refresh_selected_account_rejects_profile_target() {
        let plugin = FakePlugin {
            id: "codex",
            name: "Codex",
            accounts: Vec::new(),
            profiles: vec![config_profile_for("codex", 1, "gateway")],
            reset_outcome: None,
        };

        let err = refresh_selected_account(&plugin, "gateway").unwrap_err();

        assert!(
            err.to_string()
                .contains("refresh only supports account targets")
        );
    }

    #[test]
    fn reset_credit_cli_rejects_non_codex_provider_before_prompt() {
        let plugin = FakePlugin {
            id: "claude",
            name: "Claude",
            accounts: vec![account_status_with_alias_for("claude", 1, Some("work"))],
            profiles: Vec::new(),
            reset_outcome: Some(ResetCreditOutcome::NoCredit),
        };

        let err = consume_reset_credit_cli(&plugin, "work", true).unwrap_err();

        assert!(err.to_string().contains("supports codex accounts only"));
    }

    #[test]
    fn reset_credit_cli_consumes_confirmed_codex_account() {
        let plugin = FakePlugin {
            id: "codex",
            name: "Codex",
            accounts: vec![account_status_with_alias_for("codex", 1, Some("work"))],
            profiles: Vec::new(),
            reset_outcome: Some(ResetCreditOutcome::Reset { windows_reset: 1 }),
        };

        consume_reset_credit_cli(&plugin, "work", true).unwrap();
    }

    #[test]
    fn reset_credit_cli_rejects_profile_target() {
        let plugin = FakePlugin {
            id: "codex",
            name: "Codex",
            accounts: Vec::new(),
            profiles: vec![config_profile_for("codex", 1, "gateway")],
            reset_outcome: Some(ResetCreditOutcome::NoCredit),
        };

        let err = consume_reset_credit_cli(&plugin, "gateway", true).unwrap_err();

        assert!(
            err.to_string()
                .contains("reset-credit only supports account targets")
        );
    }

    #[test]
    fn reset_credit_requires_yes_without_tty() {
        let err = require_reset_credit_confirmation_flag(false, false).unwrap_err();
        assert!(err.to_string().contains("requires --yes"));
        require_reset_credit_confirmation_flag(true, false).unwrap();
    }

    #[test]
    fn reset_credit_outcome_messages_are_clear() {
        assert_eq!(
            reset_credit_outcome_message(&ResetCreditOutcome::NoCredit),
            "No reset credits available"
        );
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

    #[test]
    fn selector_resolution_picks_unique_account_or_profile_and_rejects_ambiguity() {
        let plugin = FakePlugin {
            id: "claude",
            name: "Claude",
            accounts: vec![
                account_status_with_alias(1, Some("work")),
                account_status_with_alias(2, Some("personal")),
            ],
            profiles: vec![config_profile(1, "gateway"), config_profile(2, "work")],
            reset_outcome: None,
        };

        assert_eq!(
            resolve_target(&plugin, "1").unwrap(),
            prismux_core::TargetResolution {
                kind: TargetKind::Account,
                target_id: "claude-account-1".to_string()
            }
        );
        assert_eq!(
            resolve_target(&plugin, "3").unwrap(),
            prismux_core::TargetResolution {
                kind: TargetKind::Profile,
                target_id: "claude-profile-gateway".to_string()
            }
        );
        assert_eq!(
            resolve_target(&plugin, "gateway").unwrap(),
            prismux_core::TargetResolution {
                kind: TargetKind::Profile,
                target_id: "claude-profile-gateway".to_string()
            }
        );
        let err = resolve_target(&plugin, "work").unwrap_err();
        assert!(err.to_string().contains("ambiguous"));
    }

    #[test]
    fn selector_resolution_uses_display_index_for_single_plugin_accounts_and_profiles() {
        let plugin = FakePlugin {
            id: "codex",
            name: "Codex",
            accounts: vec![
                account_status_with_alias(1, None),
                account_status_with_alias(2, None),
                account_status_with_alias(3, None),
            ],
            profiles: vec![config_profile_without_number("api-apikey-fun")],
            reset_outcome: None,
        };

        assert_eq!(
            resolve_target(&plugin, "4").unwrap(),
            prismux_core::TargetResolution {
                kind: TargetKind::Profile,
                target_id: "codex-profile-api-apikey-fun".to_string()
            }
        );
    }

    fn account_status_with_limits(
        number: u32,
        five_hour_percent_x100: u32,
        weekly_percent_x100: u32,
    ) -> AccountStatus {
        AccountStatus {
            account: AccountRef {
                platform: "codex".to_string(),
                local_id: format!("codex-account-{number}"),
                number,
                alias: None,
            },
            active: false,
            account_label: None,
            plan_label: None,
            auth_type: None,
            expires_at_unix: None,
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
                reset_credits: None,
                diagnostics: Vec::new(),
            }),
        }
    }

    fn account_status_with_alias(number: u32, alias: Option<&str>) -> AccountStatus {
        account_status_with_alias_for("claude", number, alias)
    }

    fn account_status_with_alias_for(
        platform: &'static str,
        number: u32,
        alias: Option<&str>,
    ) -> AccountStatus {
        AccountStatus {
            account: AccountRef {
                platform: platform.to_string(),
                local_id: format!("{platform}-account-{number}"),
                number,
                alias: alias.map(str::to_string),
            },
            active: false,
            account_label: None,
            plan_label: None,
            auth_type: None,
            expires_at_unix: None,
            availability: Availability {
                state: AvailabilityState::Unknown,
                display: "unknown".to_string(),
            },
            usage: None,
        }
    }

    fn config_profile(number: u32, name: &str) -> ConfigProfile {
        config_profile_for("claude", number, name)
    }

    fn config_profile_for(platform: &'static str, number: u32, name: &str) -> ConfigProfile {
        ConfigProfile {
            platform: prismux_core::platform_info(platform, platform),
            local_id: format!("{platform}-profile-{name}"),
            name: name.to_string(),
            active: false,
            config_path: format!("{name}.profile.json"),
            provider_id: None,
            base_url: None,
            model: None,
            number: Some(number),
            auth_type: None,
        }
    }

    fn config_profile_without_number(name: &str) -> ConfigProfile {
        ConfigProfile {
            platform: prismux_core::platform_info("codex", "Codex"),
            local_id: format!("codex-profile-{name}"),
            name: name.to_string(),
            active: false,
            config_path: format!("{name}.config.toml"),
            provider_id: None,
            base_url: None,
            model: None,
            number: None,
            auth_type: None,
        }
    }

    struct FakePlugin {
        id: &'static str,
        name: &'static str,
        accounts: Vec<AccountStatus>,
        profiles: Vec<ConfigProfile>,
        reset_outcome: Option<ResetCreditOutcome>,
    }

    impl PlatformPlugin for FakePlugin {
        fn id(&self) -> &'static str {
            self.id
        }

        fn name(&self) -> &'static str {
            self.name
        }

        fn detect(&self) -> prismux_core::Result<prismux_core::PlatformInstall> {
            Err(prismux_core::PrismuxError::Message("unused".to_string()))
        }

        fn pool_summary(&self) -> prismux_core::Result<prismux_core::PlatformPoolSummary> {
            Err(prismux_core::PrismuxError::Message("unused".to_string()))
        }

        fn current(&self) -> prismux_core::Result<Option<AccountStatus>> {
            Ok(None)
        }

        fn list_accounts(&self) -> prismux_core::Result<Vec<AccountStatus>> {
            Ok(self.accounts.clone())
        }

        fn list_configs(&self) -> prismux_core::Result<Vec<ConfigProfile>> {
            Ok(self.profiles.clone())
        }

        fn login(&self, _options: LoginOptions) -> prismux_core::Result<AccountRef> {
            Err(prismux_core::PrismuxError::Message("unused".to_string()))
        }

        fn save_current(&self, _options: SaveOptions) -> prismux_core::Result<AccountRef> {
            Err(prismux_core::PrismuxError::Message("unused".to_string()))
        }

        fn import_config(
            &self,
            _options: ImportConfigOptions,
        ) -> prismux_core::Result<prismux_core::ImportedConfig> {
            Err(prismux_core::PrismuxError::Message("unused".to_string()))
        }

        fn switch_to(&self, _selector: &str) -> prismux_core::Result<prismux_core::SwitchReport> {
            Err(prismux_core::PrismuxError::Message("unused".to_string()))
        }

        fn set_alias(&self, _selector: &str, _alias: &str) -> prismux_core::Result<AccountRef> {
            Err(prismux_core::PrismuxError::Message("unused".to_string()))
        }

        fn consume_reset_credit(
            &self,
            _selector: &str,
            _idempotency_key: &str,
        ) -> prismux_core::Result<ResetCreditOutcome> {
            self.reset_outcome
                .clone()
                .ok_or_else(|| prismux_core::PrismuxError::Message("unused".to_string()))
        }

        fn doctor(&self) -> prismux_core::Result<prismux_core::DoctorReport> {
            Err(prismux_core::PrismuxError::Message("unused".to_string()))
        }
    }
}
