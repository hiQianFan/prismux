use crate::input::{read_import_content, read_optional_import_content};
use anstream::println;
use anstyle::{AnsiColor, Style};
use anyhow::{bail, Context, Result};
use chrono::{Local, TimeZone};
use clap::{Parser, Subcommand};
use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, Attribute, Cell, Color,
    ContentArrangement, Table,
};
use inquire::Select;
use omx_core::{
    AccountRef, AccountStatus, ConfigProfile, ImportConfigOptions, LoginOptions, PlatformPlugin,
    SaveOptions, TargetCatalog, TargetKind, TargetResolution, UsageLimit, UsageSnapshot, UseReport,
};
use omx_plugin_claude::{ClaudeAccountPlugin, ClaudePlugin};
use omx_plugin_codex::CodexPlugin;
use std::{fmt, path::PathBuf};

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

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let plugins: Vec<Box<dyn PlatformPlugin>> = vec![
        Box::new(CodexPlugin::new()),
        Box::new(ClaudePlugin::new()),
        Box::new(ClaudeAccountPlugin::new()),
    ];

    match cli.command {
        Command::List { platform } => {
            if let Some(platform) = platform {
                if platform == "claude" {
                    print_aggregated_platform(
                        find_plugin(&plugins, "claude")?,
                        Some(find_plugin(&plugins, "claude-account")?),
                    )?;
                } else {
                    let plugin = find_plugin(&plugins, &platform)?;
                    print_platform_accounts(plugin)?;
                }
            } else {
                print_section("Overview");
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
                    let account_plugin = aggregated_account_plugin(plugin, &plugins)?;
                    let accounts = match account_plugin {
                        Some(plugin) => plugin.list_accounts()?,
                        None => plugin.list_accounts()?,
                    };
                    let profiles = plugin.list_configs()?;
                    let active = accounts.iter().find(|status| status.active);
                    let active_profile = profiles.iter().find(|profile| profile.active);
                    table.add_row(vec![
                        Cell::new(plugin.name()).add_attribute(Attribute::Bold),
                        active_account_cell(
                            active
                                .map(|status| account_label(&status.account))
                                .or_else(|| active_profile.map(|profile| profile.name.clone()))
                                .unwrap_or_else(|| "-".to_string()),
                        ),
                        Cell::new(accounts.len()),
                        Cell::new(profiles.len()),
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
                if platform == "claude" {
                    print_aggregated_current(
                        find_plugin(&plugins, "claude")?,
                        Some(find_plugin(&plugins, "claude-account")?),
                    )?;
                } else {
                    let plugin = find_plugin(&plugins, &platform)?;
                    match plugin.current()? {
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
                    let account_plugin = aggregated_account_plugin(plugin, &plugins)?;
                    let current = account_plugin
                        .map(PlatformPlugin::current)
                        .transpose()?
                        .flatten()
                        .or(plugin.current()?);
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
        Command::Login {
            platform,
            device_auth,
            alias,
            use_account,
        } => {
            let plugin = if platform == "claude" {
                find_plugin(&plugins, "claude-account")?
            } else {
                find_plugin(&plugins, &platform)?
            };
            let account = plugin.login(LoginOptions {
                device_auth,
                alias,
                activate: use_account,
            })?;
            let display_name = if platform == "claude" {
                "Claude Code"
            } else {
                plugin.name()
            };
            print_success(format!(
                "Imported {display_name} account {}",
                account_label(&account)
            ));
            if use_account || platform == "claude" {
                print_success(format!(
                    "Using {display_name} account {}",
                    account_label(&account)
                ));
                print_hint(format!(
                    "Restart {} if it is already running.",
                    display_name
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
            for plugin in visible_plugins(&plugins) {
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
            let requested_content =
                read_optional_import_content(file.as_deref(), clipboard, content)?;
            let (plugin, content, imported_kind) = if platform == "claude" {
                if let Some(content) = requested_content {
                    (
                        find_plugin(&plugins, "claude")?,
                        content,
                        ImportKind::Profile,
                    )
                } else {
                    (
                        find_plugin(&plugins, "claude-account")?,
                        String::new(),
                        ImportKind::Account,
                    )
                }
            } else {
                let plugin = find_plugin(&plugins, &platform)?;
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
                (plugin, content, kind)
            };
            let imported = plugin.import_config(ImportConfigOptions { name, content })?;
            let display_name = if platform == "claude" {
                "Claude Code"
            } else {
                imported.platform.name.as_str()
            };
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
            print_hint(format!(
                "List: omx list {}",
                public_platform_id(plugin.id())
            ));
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
            let account_plugin = aggregated_account_plugin(plugin, &plugins)?;
            let uses_target_catalog = account_plugin.is_some()
                || (plugin.capabilities().accounts && plugin.capabilities().profiles);
            let selector = match selector {
                Some(selector) => selector,
                None => choose_target(plugin, account_plugin, uses_target_catalog)?,
            };
            let report = if uses_target_catalog {
                use_resolved_target(plugin, account_plugin, &selector)?
            } else {
                plugin.use_target(&selector)?
            };
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
                if platform == "claude" {
                    print_doctor(find_plugin(&plugins, "claude")?)?;
                    print_doctor(find_plugin(&plugins, "claude-account")?)?;
                } else {
                    let plugin = find_plugin(&plugins, &platform)?;
                    print_doctor(plugin)?;
                }
            } else {
                for plugin in visible_plugins(&plugins) {
                    print_doctor(plugin)?;
                    if plugin.id() == "claude" {
                        print_doctor(find_plugin(&plugins, "claude-account")?)?;
                    }
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
    plugins
        .iter()
        .map(|plugin| plugin.as_ref())
        .filter(|plugin| plugin.id() != "claude-account")
}

fn aggregated_account_plugin<'a>(
    plugin: &'a dyn PlatformPlugin,
    plugins: &'a [Box<dyn PlatformPlugin>],
) -> Result<Option<&'a dyn PlatformPlugin>> {
    if plugin.id() == "claude" {
        Ok(Some(find_plugin(plugins, "claude-account")?))
    } else {
        Ok(None)
    }
}

fn public_platform_id(id: &str) -> &str {
    if id == "claude-account" {
        "claude"
    } else {
        id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportKind {
    Account,
    Profile,
}

fn use_resolved_target(
    profile_plugin: &dyn PlatformPlugin,
    account_plugin: Option<&dyn PlatformPlugin>,
    selector: &str,
) -> Result<UseReport> {
    let target = resolve_target(profile_plugin, account_plugin, selector)?;
    let account_plugin = account_plugin.unwrap_or(profile_plugin);
    match target.kind {
        TargetKind::Account => Ok(account_plugin.use_target(&target.selector)?),
        TargetKind::Profile => Ok(profile_plugin.use_target(&target.selector)?),
    }
}

fn resolve_target(
    profile_plugin: &dyn PlatformPlugin,
    account_plugin: Option<&dyn PlatformPlugin>,
    selector: &str,
) -> Result<TargetResolution> {
    let catalog = load_target_catalog(profile_plugin, account_plugin)?;
    Ok(catalog.resolve(profile_plugin.id(), selector)?)
}

fn load_target_catalog(
    profile_plugin: &dyn PlatformPlugin,
    account_plugin: Option<&dyn PlatformPlugin>,
) -> Result<TargetCatalog> {
    let accounts = match account_plugin {
        Some(plugin) => plugin.list_accounts()?,
        None => profile_plugin.list_accounts()?,
    };
    let profiles = profile_plugin.list_configs()?;
    Ok(TargetCatalog::new(accounts, profiles))
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

fn print_aggregated_platform(
    profile_plugin: &dyn PlatformPlugin,
    account_plugin: Option<&dyn PlatformPlugin>,
) -> Result<()> {
    let catalog = load_target_catalog(profile_plugin, account_plugin)?;
    print_account_section_from_statuses(profile_plugin.name(), &catalog.accounts, Some(1));
    print_platform_profiles_with_start(
        profile_plugin,
        &catalog.profiles,
        Some(catalog.accounts.len() as u32 + 1),
    );
    if catalog.accounts.is_empty() && catalog.profiles.is_empty() {
        print_hint(format!("Add account: omx login {}", profile_plugin.id()));
        print_hint(format!(
            "Add profile: omx import {} --file <path>",
            profile_plugin.id()
        ));
    }
    Ok(())
}

fn print_aggregated_current(
    profile_plugin: &dyn PlatformPlugin,
    account_plugin: Option<&dyn PlatformPlugin>,
) -> Result<()> {
    let account = account_plugin
        .map(PlatformPlugin::current)
        .transpose()?
        .flatten();
    let profile = profile_plugin
        .list_configs()?
        .into_iter()
        .find(|profile| profile.active);

    print_section(profile_plugin.name());
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
        let catalog = load_target_catalog(plugin, None)?;
        print_account_section_from_statuses(plugin.name(), &catalog.accounts, Some(1));
        print_platform_profiles_with_start(
            plugin,
            &catalog.profiles,
            Some(catalog.accounts.len() as u32 + 1),
        );
        if catalog.accounts.is_empty() && catalog.profiles.is_empty() {
            print_hint(format!("Add account: omx login {}", plugin.id()));
            print_hint(format!(
                "Add profile: omx import {} --file <path>",
                plugin.id()
            ));
        }
        return Ok(());
    }

    if capabilities.accounts {
        print_account_section(plugin.name(), plugin)?;
    }

    if capabilities.profiles {
        let profiles = plugin.list_configs()?;
        print_platform_profiles(plugin, &profiles);
    }
    Ok(())
}

fn print_account_section(title_name: &str, plugin: &dyn PlatformPlugin) -> Result<()> {
    let accounts = plugin.list_accounts()?;
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
                "No imported profiles. Run `omx import {} --file <path>`.",
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
        status_badge(&usage_status_display(usage, &status.availability)),
    ]
}

fn choose_target(
    profile_plugin: &dyn PlatformPlugin,
    account_plugin: Option<&dyn PlatformPlugin>,
    use_display_selector: bool,
) -> Result<String> {
    let accounts = match account_plugin {
        Some(plugin) => plugin.list_accounts()?,
        None => profile_plugin.list_accounts()?,
    };
    let profiles = profile_plugin.list_configs()?;
    if accounts.is_empty() && profiles.is_empty() {
        bail!(
            "no saved accounts or profiles for platform `{}`",
            profile_plugin.id()
        );
    }

    let catalog = TargetCatalog::new(accounts, profiles);
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
                diagnostics: Vec::new(),
            }),
        };

        let choice = target_choice_from_account(&status, 1, false);

        assert_eq!(choice.selector, "2");
        assert!(choice
            .label
            .starts_with("* #1 account work · team@example.com · Pro"));
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
            vec!["*", "#", "Alias", "Account", "Plan", "5h", "Weekly", "Status"]
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
        let account_plugin = FakePlugin {
            id: "claude-account",
            name: "Claude Account",
            accounts: vec![
                account_status_with_alias(1, Some("work")),
                account_status_with_alias(2, Some("personal")),
            ],
            profiles: Vec::new(),
        };
        let profile_plugin = FakePlugin {
            id: "claude",
            name: "Claude",
            accounts: Vec::new(),
            profiles: vec![config_profile(1, "gateway"), config_profile(2, "work")],
        };

        assert_eq!(
            resolve_target(&profile_plugin, Some(&account_plugin), "1").unwrap(),
            TargetResolution {
                kind: TargetKind::Account,
                selector: "1".to_string()
            }
        );
        assert_eq!(
            resolve_target(&profile_plugin, Some(&account_plugin), "3").unwrap(),
            TargetResolution {
                kind: TargetKind::Profile,
                selector: "1".to_string()
            }
        );
        assert_eq!(
            resolve_target(&profile_plugin, Some(&account_plugin), "gateway").unwrap(),
            TargetResolution {
                kind: TargetKind::Profile,
                selector: "1".to_string()
            }
        );
        let err = resolve_target(&profile_plugin, Some(&account_plugin), "work").unwrap_err();
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
        };

        assert_eq!(
            resolve_target(&plugin, None, "4").unwrap(),
            TargetResolution {
                kind: TargetKind::Profile,
                selector: "api-apikey-fun".to_string()
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
                diagnostics: Vec::new(),
            }),
        }
    }

    fn account_status_with_alias(number: u32, alias: Option<&str>) -> AccountStatus {
        AccountStatus {
            account: AccountRef {
                platform: "claude-account".to_string(),
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
        ConfigProfile {
            platform: omx_core::platform_info("claude", "Claude"),
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
            platform: omx_core::platform_info("codex", "Codex"),
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
    }

    impl PlatformPlugin for FakePlugin {
        fn id(&self) -> &'static str {
            self.id
        }

        fn name(&self) -> &'static str {
            self.name
        }

        fn detect(&self) -> omx_core::Result<omx_core::PlatformInstall> {
            Err(omx_core::OpenMuxError::Message("unused".to_string()))
        }

        fn pool_summary(&self) -> omx_core::Result<omx_core::PlatformPoolSummary> {
            Err(omx_core::OpenMuxError::Message("unused".to_string()))
        }

        fn current(&self) -> omx_core::Result<Option<AccountStatus>> {
            Ok(None)
        }

        fn list_accounts(&self) -> omx_core::Result<Vec<AccountStatus>> {
            Ok(self.accounts.clone())
        }

        fn list_configs(&self) -> omx_core::Result<Vec<ConfigProfile>> {
            Ok(self.profiles.clone())
        }

        fn login(&self, _options: LoginOptions) -> omx_core::Result<AccountRef> {
            Err(omx_core::OpenMuxError::Message("unused".to_string()))
        }

        fn save_current(&self, _options: SaveOptions) -> omx_core::Result<AccountRef> {
            Err(omx_core::OpenMuxError::Message("unused".to_string()))
        }

        fn import_config(
            &self,
            _options: ImportConfigOptions,
        ) -> omx_core::Result<omx_core::ImportedConfig> {
            Err(omx_core::OpenMuxError::Message("unused".to_string()))
        }

        fn switch_to(&self, _selector: &str) -> omx_core::Result<omx_core::SwitchReport> {
            Err(omx_core::OpenMuxError::Message("unused".to_string()))
        }

        fn set_alias(&self, _selector: &str, _alias: &str) -> omx_core::Result<AccountRef> {
            Err(omx_core::OpenMuxError::Message("unused".to_string()))
        }

        fn doctor(&self) -> omx_core::Result<omx_core::DoctorReport> {
            Err(omx_core::OpenMuxError::Message("unused".to_string()))
        }
    }
}
