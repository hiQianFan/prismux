use crate::input::{read_import_content, read_optional_import_content};
use anstream::println;
use anstyle::{AnsiColor, Style};
use anyhow::{Context, Result, bail};
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::{
    Attribute, Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS,
    presets::UTF8_FULL_CONDENSED,
};
use inquire::Select;
use omx_core::{
    AccountRef, AccountStatus, ConfigProfile, ImportConfigOptions, LoginOptions, PlatformPlugin,
    RemoveReport, SaveOptions, StateStore, TargetKind, UsageAccounting, UsageCoverage,
    UsageFreshness, UsageGroupBy, UsageLimit, UsagePeriod, UsageQuery, UsageReport,
    UsageReportScan, UsageScanBudget, UsageScanDiagnostic, UsageScanOptions, UsageSnapshot,
    UsageSummary, UsageSummaryQuery, UseReport,
    storage::{state_root, unix_now},
};
use omx_plugin_claude::ClaudePlugin;
use omx_plugin_codex::CodexPlugin;
use omx_usage_tokscale::TokscaleUsageBackend;
use serde_json::{Value, json};
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
    /// Refresh provider quota snapshots for one platform.
    Refresh {
        /// Platform id, for example: codex.
        platform: String,
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
    /// Remove a managed account or profile from OpenMux.
    Remove {
        /// Platform id, for example: codex.
        platform: String,
        /// Account/profile number, alias, or profile name. If omitted, OpenMux will ask you to choose.
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
    /// Run platform diagnostics.
    Doctor {
        /// Optional platform id, for example: codex.
        platform: Option<String>,
    },
    /// Show parsed local token usage.
    Usage {
        /// Optional client id, for example: codex, claude, or gemini.
        client: Option<String>,
        /// Common time window preset.
        #[arg(long, value_enum)]
        period: Option<UsagePeriodArg>,
        /// Start of the local date window as YYYY-MM-DD or a Unix timestamp.
        #[arg(long)]
        since: Option<String>,
        /// End of the local date window as YYYY-MM-DD or a Unix timestamp. Date values are exclusive of the next day.
        #[arg(long)]
        until: Option<String>,
        /// Group usage rows by client, local day, or model.
        #[arg(long, value_enum)]
        group_by: Option<UsageGroupByArg>,
        /// Show full token accounting columns.
        #[arg(long)]
        details: bool,
        /// Print a versioned JSON document.
        #[arg(long)]
        json: bool,
        /// Skip local log scan and query the existing SQLite cache only.
        #[arg(long)]
        no_scan: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum UsagePeriodArg {
    Today,
    #[value(name = "7d")]
    SevenDays,
    #[value(name = "30d")]
    ThirtyDays,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum UsageGroupByArg {
    Client,
    Day,
    Model,
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
                let dashboard =
                    omx_app::dashboard_view(&plugins, omx_app::MenubarQuery::default(), None)?;
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
                    let accounts = dashboard
                        .accounts
                        .accounts
                        .iter()
                        .filter(|account| account.provider == plugin.id())
                        .collect::<Vec<_>>();
                    let profiles = dashboard
                        .accounts
                        .profiles
                        .iter()
                        .filter(|profile| profile.provider == plugin.id())
                        .collect::<Vec<_>>();
                    let active_label = accounts
                        .iter()
                        .find(|account| account.active)
                        .map(|account| account.display_label.clone())
                        .or_else(|| {
                            profiles
                                .iter()
                                .find(|profile| profile.active)
                                .map(|profile| profile.display_label.clone())
                        })
                        .unwrap_or_else(|| "-".to_string());
                    let attention = dashboard
                        .provider_views
                        .iter()
                        .find(|view| view.provider == plugin.id())
                        .filter(|view| {
                            matches!(
                                view.status,
                                omx_app::MenubarAccountStatus::Limited
                                    | omx_app::MenubarAccountStatus::Exhausted
                                    | omx_app::MenubarAccountStatus::Stale
                                    | omx_app::MenubarAccountStatus::Unavailable
                            )
                        })
                        .map(|_| 1)
                        .unwrap_or_default();
                    table.add_row(vec![
                        Cell::new(plugin.name()).add_attribute(Attribute::Bold),
                        active_account_cell(active_label),
                        Cell::new(accounts.len()),
                        Cell::new(profiles.len()),
                        usage_cell(&menubar_overall_availability(&accounts)),
                        usage_cell(&menubar_window_availability(&accounts, 18_000)),
                        pool_status_cell(attention),
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
                    match omx_app::active_account_status(plugin)? {
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
                    let current = omx_app::active_account_status(plugin)?;
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
        Command::Refresh { platform } => {
            let plugin = find_plugin(&plugins, &platform)?;
            let accounts = plugin.refresh_accounts()?;
            print_account_section_from_statuses(plugin.name(), &accounts, None);
            print_hint(
                "Proxy: set OMUX_HTTPS_PROXY, HTTPS_PROXY, or ALL_PROXY before running refresh.",
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
            let account_is_active = if platform == "claude" {
                true
            } else {
                StateStore::open(&state_root()?)?
                    .active_account(plugin.id())?
                    .is_some_and(|active| active.local_id == account.local_id)
            };
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
            let dashboard =
                omx_app::dashboard_view(&plugins, omx_app::MenubarQuery::default(), None)?;
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
            print_hint(format!("List: omx list {}", plugin.id()));
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
        Command::Doctor { platform } => {
            if let Some(platform) = platform {
                print_doctor(find_plugin(&plugins, &platform)?)?;
            } else {
                for plugin in visible_plugins(&plugins) {
                    print_doctor(plugin)?;
                }
            }
        }
        Command::Usage {
            client,
            period,
            since,
            until,
            group_by,
            details,
            json,
            no_scan,
        } => {
            print_usage(UsageCommandOptions {
                client,
                period,
                since,
                until,
                group_by,
                details,
                json_output: json,
                no_scan,
            })?;
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

struct UsageCommandOptions {
    client: Option<String>,
    period: Option<UsagePeriodArg>,
    since: Option<String>,
    until: Option<String>,
    group_by: Option<UsageGroupByArg>,
    details: bool,
    json_output: bool,
    no_scan: bool,
}

fn print_usage(options: UsageCommandOptions) -> Result<()> {
    let window = usage_window(
        options.period,
        options.since.as_deref(),
        options.until.as_deref(),
    )?;
    let store = StateStore::open(&state_root()?)?;
    let report = usage_report(
        &store,
        options.client,
        window,
        options.group_by,
        options.details,
        options.no_scan,
    )?;

    if options.json_output {
        print_usage_json(&report)?;
    } else {
        print_usage_table(&report);
    }

    Ok(())
}

fn usage_report(
    store: &StateStore,
    client: Option<String>,
    window: UsageWindow,
    group_by_arg: Option<UsageGroupByArg>,
    details: bool,
    no_scan: bool,
) -> Result<UsageReport> {
    let mut diagnostics = Vec::new();
    let mut scanned_events = 0_usize;
    let mut inserted_events = 0_usize;
    let generated_at_unix = unix_now();

    if !no_scan {
        match TokscaleUsageBackend::new().scan(UsageScanOptions {
            clients: client.iter().cloned().collect(),
            since_unix: Some(window.since_unix),
            until_unix: Some(window.until_unix),
            budget: UsageScanBudget::default(),
        }) {
            Ok(report) => {
                scanned_events = report.events.len();
                diagnostics.extend(report.diagnostics);
                inserted_events = store.ingest_usage_events(&report.events, None, unix_now())?;
            }
            Err(err) => diagnostics.push(UsageScanDiagnostic {
                client: client.clone(),
                source_kind: Some("tokscale-local".to_string()),
                code: "scan_failed".to_string(),
                message: err.to_string(),
            }),
        }
    }

    let group_by = usage_group_by(group_by_arg, &window.period);
    let summary_query = UsageSummaryQuery {
        client: client.clone(),
        since_unix: Some(window.since_unix),
        until_unix: Some(window.until_unix),
        group_by_local_day: matches!(group_by, UsageGroupBy::Day),
        local_day_offset_seconds: Local::now().offset().local_minus_utc(),
        group_by_model_provider: matches!(group_by, UsageGroupBy::Model),
        group_by_model: matches!(group_by, UsageGroupBy::Model),
        ..UsageSummaryQuery::default()
    };
    let summaries = store.usage_summaries_by(summary_query)?;
    let model_summaries = store.usage_summaries_by(UsageSummaryQuery {
        client: client.clone(),
        since_unix: Some(window.since_unix),
        until_unix: Some(window.until_unix),
        group_by_local_day: matches!(group_by, UsageGroupBy::Day),
        local_day_offset_seconds: Local::now().offset().local_minus_utc(),
        group_by_model: true,
        ..UsageSummaryQuery::default()
    })?;
    let groups = usage_groups(group_by, summaries, &model_summaries);
    let totals = usage_total(&groups);
    let requested_clients = client.iter().cloned().collect::<Vec<_>>();
    let available_clients = groups
        .iter()
        .map(|summary| summary.client.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let missing_clients = if let Some(client) = client.as_ref() {
        if available_clients.iter().any(|value| value == client) {
            Vec::new()
        } else {
            vec![client.clone()]
        }
    } else {
        Vec::new()
    };
    let coverage_status = if diagnostics.is_empty() {
        if totals.event_count == 0 {
            "empty"
        } else {
            "complete"
        }
    } else if totals.event_count == 0 {
        "unavailable"
    } else {
        "partial"
    };
    let total_cost_status = totals.cost_status.clone();

    Ok(UsageReport {
        query: UsageQuery {
            client,
            since_unix: Some(window.since_unix),
            until_unix: Some(window.until_unix),
            period: window.period,
            group_by,
            details,
        },
        totals,
        groups,
        freshness: UsageFreshness {
            generated_at_unix,
            last_scan_at_unix: (!no_scan).then_some(generated_at_unix),
            stale: no_scan || !diagnostics.is_empty(),
        },
        coverage: UsageCoverage {
            requested_clients,
            available_clients,
            missing_clients,
            status: coverage_status.to_string(),
        },
        accounting: UsageAccounting {
            quality: omx_core::UsageDataQuality::Parsed,
            cost_status: total_cost_status,
            note: usage_accounting_note().to_string(),
        },
        diagnostics,
        scan: UsageReportScan {
            enabled: !no_scan,
            scanned_events,
            inserted_events,
        },
    })
}

fn usage_groups(
    group_by: UsageGroupBy,
    summaries: Vec<UsageSummary>,
    model_summaries: &[UsageSummary],
) -> Vec<UsageSummary> {
    if matches!(group_by, UsageGroupBy::Client) {
        return summaries
            .into_iter()
            .map(|mut summary| {
                summary.top_model = top_model_for_client(model_summaries, &summary.client);
                summary
            })
            .collect();
    }
    if matches!(group_by, UsageGroupBy::Model) {
        return summaries
            .into_iter()
            .map(|mut summary| {
                summary.top_model = summary.model.clone();
                summary
            })
            .collect();
    }

    let mut by_day = std::collections::BTreeMap::<String, UsageSummary>::new();
    for summary in summaries {
        let day = summary
            .local_day
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        by_day
            .entry(day.clone())
            .or_insert_with(|| {
                let mut total = UsageSummary::empty("all");
                total.local_day = Some(day);
                total
            })
            .add(&summary);
    }
    by_day
        .into_values()
        .map(|mut summary| {
            if let Some(day) = summary.local_day.as_deref() {
                summary.top_model = top_model_for_day(model_summaries, day);
            }
            summary
        })
        .collect()
}

fn top_model_for_client(model_summaries: &[UsageSummary], client: &str) -> Option<String> {
    model_summaries
        .iter()
        .filter(|summary| summary.client == client)
        .max_by_key(|summary| summary.normalized_total_tokens)
        .and_then(|summary| summary.model.clone())
}

fn top_model_for_day(model_summaries: &[UsageSummary], day: &str) -> Option<String> {
    model_summaries
        .iter()
        .filter(|summary| summary.local_day.as_deref() == Some(day))
        .max_by_key(|summary| summary.normalized_total_tokens)
        .and_then(|summary| summary.model.clone())
}

fn usage_total(summaries: &[UsageSummary]) -> UsageSummary {
    let mut total = UsageSummary::empty("all");
    for summary in summaries {
        total.add(summary);
    }
    total
}

#[derive(Debug, Clone)]
struct UsageWindow {
    since_unix: i64,
    until_unix: i64,
    period: UsagePeriod,
}

fn usage_window(
    period: Option<UsagePeriodArg>,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<UsageWindow> {
    if (since.is_some() || until.is_some()) && period.is_some() {
        bail!("usage --period cannot be combined with --since or --until");
    }
    let today = Local::now().date_naive();
    let (since_unix, until_unix, period) = if since.is_some() || until.is_some() {
        (
            match since {
                Some(value) => parse_usage_boundary(value, UsageBoundary::Start)?,
                None => local_date_start_unix(today)?,
            },
            match until {
                Some(value) => parse_usage_boundary(value, UsageBoundary::End)?,
                None => local_date_start_unix(today.succ_opt().context("invalid local date")?)?,
            },
            UsagePeriod::Custom,
        )
    } else {
        usage_period_window(period.unwrap_or(UsagePeriodArg::SevenDays), today)?
    };
    if since_unix >= until_unix {
        bail!("usage window must have --since earlier than --until");
    }

    Ok(UsageWindow {
        since_unix,
        until_unix,
        period,
    })
}

fn usage_period_window(
    period: UsagePeriodArg,
    today: NaiveDate,
) -> Result<(i64, i64, UsagePeriod)> {
    let until = local_date_start_unix(today.succ_opt().context("invalid local date")?)?;
    match period {
        UsagePeriodArg::Today => Ok((local_date_start_unix(today)?, until, UsagePeriod::Today)),
        UsagePeriodArg::SevenDays => Ok((
            local_date_start_unix(today - Duration::days(6))?,
            until,
            UsagePeriod::SevenDays,
        )),
        UsagePeriodArg::ThirtyDays => Ok((
            local_date_start_unix(today - Duration::days(29))?,
            until,
            UsagePeriod::ThirtyDays,
        )),
        UsagePeriodArg::All => Ok((0, i64::MAX, UsagePeriod::All)),
    }
}

fn usage_group_by(group_by: Option<UsageGroupByArg>, period: &UsagePeriod) -> UsageGroupBy {
    match group_by {
        Some(UsageGroupByArg::Client) => UsageGroupBy::Client,
        Some(UsageGroupByArg::Day) => UsageGroupBy::Day,
        Some(UsageGroupByArg::Model) => UsageGroupBy::Model,
        None if matches!(period, UsagePeriod::Today | UsagePeriod::Custom) => UsageGroupBy::Client,
        None => UsageGroupBy::Day,
    }
}

#[derive(Debug, Clone, Copy)]
enum UsageBoundary {
    Start,
    End,
}

fn parse_usage_boundary(value: &str, boundary: UsageBoundary) -> Result<i64> {
    if let Ok(timestamp) = value.parse::<i64>() {
        return Ok(timestamp);
    }

    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").with_context(|| {
        format!("invalid usage date `{value}`, expected YYYY-MM-DD or Unix timestamp")
    })?;
    match boundary {
        UsageBoundary::Start => local_date_start_unix(date),
        UsageBoundary::End => local_date_start_unix(date.succ_opt().context("invalid usage date")?),
    }
}

fn local_date_start_unix(date: NaiveDate) -> Result<i64> {
    Local
        .with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
        .single()
        .map(|time| time.timestamp())
        .context("local date boundary is ambiguous or invalid")
}

fn display_usage_timestamp(timestamp: i64) -> String {
    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .map(|time| time.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| timestamp.to_string())
}

fn print_usage_table(report: &UsageReport) {
    print_section(format!(
        "Usage: {} · {} to {} · {}",
        report.query.client.as_deref().unwrap_or("all"),
        display_usage_timestamp(report.query.since_unix.unwrap_or_default()),
        display_usage_timestamp(report.query.until_unix.unwrap_or_default()),
        Local::now().offset()
    ));
    println!(
        "Total: {} tokens · cost {} · coverage {}",
        report.totals.normalized_total_tokens,
        usage_cost_display(&report.totals),
        report.coverage.status
    );
    if !report.coverage.available_clients.is_empty() {
        print_hint(format!(
            "clients: {}",
            report.coverage.available_clients.join(", ")
        ));
    }
    if !report.scan.enabled {
        print_hint("scan skipped; showing cached usage only");
    } else {
        print_hint(format!(
            "scanned {} events, inserted {} new events",
            report.scan.scanned_events, report.scan.inserted_events
        ));
    }
    print_hint(usage_accounting_note());

    let mut table = view_table();
    if report.query.details {
        let mut header = vec![header_cell(usage_group_label(report.query.group_by))];
        if !matches!(report.query.group_by, UsageGroupBy::Model) {
            header.push(header_cell("Top Model"));
        }
        header.extend(vec![
            header_cell("Input"),
            header_cell("Output"),
            header_cell("Cache Read"),
            header_cell("Cache Write"),
            header_cell("Reasoning"),
            header_cell("Total"),
            header_cell("Provider Total"),
            header_cell("Cost"),
            header_cell("Quality"),
            header_cell("Events"),
            header_cell("Of Total"),
        ]);
        table.set_header(header);
        for summary in &report.groups {
            let mut row = vec![
                Cell::new(usage_group_value(report.query.group_by, summary))
                    .add_attribute(Attribute::Bold),
            ];
            if !matches!(report.query.group_by, UsageGroupBy::Model) {
                row.push(text_or_empty_cell(summary.top_model.as_deref()));
            }
            row.extend(vec![
                Cell::new(summary.tokens.input),
                Cell::new(summary.tokens.output),
                Cell::new(summary.tokens.cache_read),
                Cell::new(summary.tokens.cache_write),
                Cell::new(summary.tokens.reasoning),
                Cell::new(summary.normalized_total_tokens),
                summary
                    .provider_total_tokens
                    .map(Cell::new)
                    .unwrap_or_else(|| muted_cell("-")),
                Cell::new(usage_cost_display(summary)),
                Cell::new(usage_quality_display(summary)),
                Cell::new(summary.event_count),
                Cell::new(usage_share(summary, &report.totals)),
            ]);
            table.add_row(row);
        }
    } else {
        let mut header = vec![header_cell(usage_group_label(report.query.group_by))];
        if !matches!(report.query.group_by, UsageGroupBy::Model) {
            header.push(header_cell("Top Model"));
        }
        header.extend(vec![
            header_cell("In"),
            header_cell("Out"),
            header_cell("Total Tokens"),
            header_cell("Cost"),
            header_cell("Events"),
            header_cell("Of Total"),
        ]);
        table.set_header(header);
        for summary in &report.groups {
            let mut row = vec![
                Cell::new(usage_group_value(report.query.group_by, summary))
                    .add_attribute(Attribute::Bold),
            ];
            if !matches!(report.query.group_by, UsageGroupBy::Model) {
                row.push(text_or_empty_cell(summary.top_model.as_deref()));
            }
            row.extend(vec![
                Cell::new(summary.tokens.input),
                Cell::new(summary.tokens.output),
                Cell::new(summary.normalized_total_tokens),
                Cell::new(usage_cost_display(summary)),
                Cell::new(summary.event_count),
                Cell::new(usage_share(summary, &report.totals)),
            ]);
            table.add_row(row);
        }
    }

    if report.groups.is_empty() {
        println!("{}", muted("No usage events found for this window."));
    } else {
        println!("{table}");
    }

    for diagnostic in &report.diagnostics {
        print_hint(format!(
            "usage diagnostic [{}]: {}",
            diagnostic.code,
            sanitized_usage_diagnostic_message(&diagnostic.message)
        ));
    }
}

fn print_usage_json(report: &UsageReport) -> Result<()> {
    let payload = usage_json_payload(report);
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn usage_json_payload(report: &UsageReport) -> Value {
    let groups = report
        .groups
        .iter()
        .map(usage_summary_json)
        .collect::<Vec<_>>();
    let diagnostics = report
        .diagnostics
        .iter()
        .map(|diagnostic| {
            json!({
                "client": diagnostic.client,
                "source_kind": diagnostic.source_kind,
                "code": diagnostic.code,
                "message": sanitized_usage_diagnostic_message(&diagnostic.message),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "schema_version": 1,
        "generated_at_unix": unix_now(),
        "timezone": Local::now().offset().to_string(),
        "notes": {
            "usage": usage_accounting_note(),
            "cost": "cost is optional and may be missing or estimated unless reported by the source",
        },
        "window": {
            "period": usage_period_label(&report.query.period),
            "since_unix": report.query.since_unix,
            "until_unix": report.query.until_unix,
            "since": report.query.since_unix.map(display_usage_timestamp),
            "until": report.query.until_unix.map(display_usage_timestamp),
        },
        "scan": {
            "enabled": report.scan.enabled,
            "scanned_events": report.scan.scanned_events,
            "inserted_events": report.scan.inserted_events,
        },
        "filter": {
            "client": report.query.client,
        },
        "group_by": usage_group_by_label(report.query.group_by),
        "quality": "parsed",
        "totals": usage_summary_json(&report.totals),
        "groups": groups,
        "clients": report.groups.iter().filter(|summary| summary.client != "all").map(usage_summary_json).collect::<Vec<_>>(),
        "freshness": {
            "generated_at_unix": report.freshness.generated_at_unix,
            "last_scan_at_unix": report.freshness.last_scan_at_unix,
            "stale": report.freshness.stale,
        },
        "coverage": {
            "status": report.coverage.status,
            "requested_clients": report.coverage.requested_clients,
            "available_clients": report.coverage.available_clients,
            "missing_clients": report.coverage.missing_clients,
        },
        "accounting": {
            "quality": usage_quality_label(&report.accounting.quality),
            "cost_status": usage_cost_status_label(&report.accounting.cost_status),
            "note": report.accounting.note,
        },
        "diagnostics": diagnostics,
    })
}

fn usage_summary_json(summary: &UsageSummary) -> Value {
    json!({
        "client": summary.client,
        "local_day": summary.local_day,
        "model_provider": summary.model_provider,
        "model": summary.model,
        "top_model": summary.top_model,
        "tokens": {
            "input": summary.tokens.input,
            "output": summary.tokens.output,
            "cache_read": summary.tokens.cache_read,
            "cache_write": summary.tokens.cache_write,
            "cache_write_5m": summary.tokens.cache_write_5m,
            "cache_write_1h": summary.tokens.cache_write_1h,
            "reasoning": summary.tokens.reasoning,
            "extra": summary.tokens.extra,
            "normalized_total": summary.normalized_total_tokens,
            "provider_total": summary.provider_total_tokens,
        },
        "cost": {
            "status": usage_cost_status_display(summary),
            "estimated_usd": summary.estimated_cost_usd,
        },
        "quality": usage_quality_display(summary),
        "event_count": summary.event_count,
    })
}

fn usage_cost_display(summary: &UsageSummary) -> String {
    match summary.estimated_cost_usd {
        Some(cost) => format!("${cost:.4} {}", usage_cost_status_display(summary)),
        None => usage_cost_status_display(summary).to_string(),
    }
}

fn usage_share(summary: &UsageSummary, total: &UsageSummary) -> String {
    if total.normalized_total_tokens == 0 {
        return "-".to_string();
    }
    format!(
        "{}%",
        summary.normalized_total_tokens.saturating_mul(100) / total.normalized_total_tokens
    )
}

fn usage_group_label(group_by: UsageGroupBy) -> &'static str {
    match group_by {
        UsageGroupBy::Client => "Client",
        UsageGroupBy::Day => "Day",
        UsageGroupBy::Model => "Model",
    }
}

fn usage_group_value(group_by: UsageGroupBy, summary: &UsageSummary) -> String {
    match group_by {
        UsageGroupBy::Client => summary.client.clone(),
        UsageGroupBy::Day => summary
            .local_day
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        UsageGroupBy::Model => summary
            .model
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
    }
}

fn usage_period_label(period: &UsagePeriod) -> &'static str {
    match period {
        UsagePeriod::Today => "today",
        UsagePeriod::SevenDays => "7d",
        UsagePeriod::ThirtyDays => "30d",
        UsagePeriod::All => "all",
        UsagePeriod::Custom => "custom",
    }
}

fn usage_group_by_label(group_by: UsageGroupBy) -> &'static str {
    match group_by {
        UsageGroupBy::Client => "client",
        UsageGroupBy::Day => "day",
        UsageGroupBy::Model => "model",
    }
}

fn usage_accounting_note() -> &'static str {
    "parsed local usage; not provider billing or exact quota accounting"
}

fn usage_cost_status_display(summary: &UsageSummary) -> &'static str {
    usage_cost_status_label(&summary.cost_status)
}

fn usage_cost_status_label(status: &omx_core::CostStatus) -> &'static str {
    match status {
        omx_core::CostStatus::ProviderReported => "provider_reported",
        omx_core::CostStatus::Estimated => "estimated",
        omx_core::CostStatus::Missing => "missing",
        omx_core::CostStatus::Mixed => "mixed",
    }
}

fn usage_quality_display(summary: &UsageSummary) -> &'static str {
    usage_quality_label(&summary.quality)
}

fn usage_quality_label(quality: &omx_core::UsageDataQuality) -> &'static str {
    match quality {
        omx_core::UsageDataQuality::Parsed => "parsed",
    }
}

fn sanitized_usage_diagnostic_message(message: &str) -> String {
    let lower = message.to_ascii_lowercase();
    let sensitive_markers = [
        "access_token",
        "access token",
        "refresh_token",
        "refresh token",
        "api_key",
        "api key",
        "x-api-key",
        "authorization:",
        "bearer ",
        "auth payload",
        "raw prompt",
        "raw response",
        "raw log line",
        "sk-",
    ];
    if sensitive_markers
        .iter()
        .any(|marker| lower.contains(marker))
    {
        "[redacted sensitive diagnostic]".to_string()
    } else {
        message.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportKind {
    Account,
    Profile,
}

fn use_resolved_target(plugin: &dyn PlatformPlugin, selector: &str) -> Result<omx_core::UseReport> {
    Ok(omx_app::use_resolved_target(plugin, selector)?)
}

fn remove_resolved_target(
    plugin: &dyn PlatformPlugin,
    selector: &str,
) -> Result<omx_core::RemoveReport> {
    Ok(omx_app::remove_resolved_target(plugin, selector)?)
}

fn resolve_target(
    plugin: &dyn PlatformPlugin,
    selector: &str,
) -> Result<omx_core::TargetResolution> {
    Ok(omx_app::resolve_target(plugin, selector)?)
}

fn load_target_catalog(plugin: &dyn PlatformPlugin) -> Result<omx_core::TargetCatalog> {
    Ok(omx_app::target_catalog(plugin)?)
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
    let account = omx_app::active_account_status(plugin)?;
    let profile = omx_app::config_profiles(plugin)?
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
        let profiles = omx_app::config_profiles(plugin)?;
        print_platform_profiles(plugin, &profiles);
    }
    Ok(())
}

fn print_account_section(title_name: &str, plugin: &dyn PlatformPlugin) -> Result<()> {
    let accounts = omx_app::account_statuses(plugin)?;
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

fn menubar_overall_availability(accounts: &[&omx_app::MenubarAccount]) -> String {
    average_percent(accounts.iter().filter_map(|account| {
        account
            .quota
            .as_ref()
            .and_then(|quota| quota.primary_window.as_ref())
            .and_then(|window| window.remaining_percent_x100)
            .map(|percent| percent as f64 / 100.0)
    }))
}

fn menubar_window_availability(
    accounts: &[&omx_app::MenubarAccount],
    window_seconds: u64,
) -> String {
    average_percent(accounts.iter().filter_map(|account| {
        account
            .quota
            .as_ref()
            .and_then(|quota| {
                quota
                    .windows
                    .iter()
                    .find(|window| window.window_seconds == Some(window_seconds))
            })
            .and_then(|window| window.remaining_percent_x100)
            .map(|percent| percent as f64 / 100.0)
    }))
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

fn usage_refresh_display(usage: Option<&UsageSnapshot>) -> String {
    usage
        .and_then(|usage| usage.refreshed_at_unix)
        .map(reset_time_display)
        .unwrap_or_else(|| "-".to_string())
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

fn login_use_hint(platform: &str, account: &AccountRef) -> String {
    let selector = account
        .alias
        .clone()
        .unwrap_or_else(|| account.number.to_string());
    format!("Account imported but not active. Use it with: omx use {platform} {selector}")
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
    use clap::CommandFactory;
    use omx_core::{
        Availability, AvailabilityState, CostStatus, UsageDataQuality, UsageLimitKind,
        UsageLimitScope, UsageSource, UsageTokenBreakdown,
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
            "Account imported but not active. Use it with: omx use codex work"
        );

        account.alias = None;
        assert_eq!(
            login_use_hint("codex", &account),
            "Account imported but not active. Use it with: omx use codex 2"
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
        assert!(help.contains("usage"));
        assert!(help.contains("Show parsed local token usage"));
    }

    #[test]
    fn usage_date_until_is_exclusive_next_local_day() {
        let since = parse_usage_boundary("2026-06-23", UsageBoundary::Start).unwrap();
        let until = parse_usage_boundary("2026-06-23", UsageBoundary::End).unwrap();

        assert_eq!(until - since, 86_400);
    }

    #[test]
    fn usage_unix_boundary_is_passthrough() {
        assert_eq!(
            parse_usage_boundary("1782144000", UsageBoundary::Start).unwrap(),
            1_782_144_000
        );
        assert_eq!(
            parse_usage_boundary("1782144000", UsageBoundary::End).unwrap(),
            1_782_144_000
        );
    }

    #[test]
    fn missing_usage_cost_does_not_render_zero_dollars() {
        let summary = UsageSummary {
            client: "codex".to_string(),
            local_day: None,
            local_hour: None,
            model_provider: None,
            model: None,
            top_model: None,
            project_path: None,
            session_id: None,
            tokens: UsageTokenBreakdown::default(),
            normalized_total_tokens: 0,
            provider_total_tokens: None,
            estimated_cost_usd: None,
            cost_status: CostStatus::Missing,
            quality: UsageDataQuality::Parsed,
            event_count: 1,
        };

        assert_eq!(usage_cost_display(&summary), "missing");
    }

    #[test]
    fn usage_json_payload_contains_versioned_empty_no_scan_shape() {
        let payload = usage_json_payload(&test_usage_report(Vec::new(), Vec::new(), true));

        assert_eq!(payload["schema_version"], 1);
        assert_eq!(
            payload["notes"]["usage"],
            "parsed local usage; not provider billing or exact quota accounting"
        );
        assert_eq!(
            payload["notes"]["cost"],
            "cost is optional and may be missing or estimated unless reported by the source"
        );
        assert_eq!(payload["window"]["since_unix"], 1_782_144_000);
        assert_eq!(payload["window"]["until_unix"], 1_782_230_400);
        assert_eq!(payload["totals"]["tokens"]["normalized_total"], 0);
        assert_eq!(payload["scan"]["enabled"], false);
        assert_eq!(payload["clients"].as_array().unwrap().len(), 0);
        assert_eq!(payload["groups"].as_array().unwrap().len(), 0);
        assert_eq!(payload["diagnostics"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn usage_json_payload_redacts_sensitive_diagnostics() {
        let diagnostics = vec![
            UsageScanDiagnostic {
                client: Some("codex".to_string()),
                source_kind: Some("tokscale-local".to_string()),
                code: "parse_error".to_string(),
                message:
                    "raw prompt hello; access_token=secret; api_key=sk-secret; raw response bye"
                        .to_string(),
            },
            UsageScanDiagnostic {
                client: Some("gemini".to_string()),
                source_kind: Some("tokscale-local".to_string()),
                code: "missing_source".to_string(),
                message: "local usage source not found".to_string(),
            },
        ];

        let payload = usage_json_payload(&test_usage_report(Vec::new(), diagnostics, false));
        let rendered = serde_json::to_string(&payload).unwrap();

        assert!(rendered.contains("[redacted sensitive diagnostic]"));
        assert!(rendered.contains("local usage source not found"));
        assert!(!rendered.contains("hello"));
        assert!(!rendered.contains("secret"));
        assert!(!rendered.contains("sk-secret"));
        assert!(!rendered.contains("bye"));
    }

    #[test]
    fn usage_json_payload_preserves_partial_coverage() {
        let summary = UsageSummary {
            client: "codex".to_string(),
            local_day: None,
            local_hour: None,
            model_provider: None,
            model: None,
            top_model: Some("gpt-5".to_string()),
            project_path: None,
            session_id: None,
            tokens: UsageTokenBreakdown {
                input: 2,
                output: 3,
                ..UsageTokenBreakdown::default()
            },
            normalized_total_tokens: 5,
            provider_total_tokens: None,
            estimated_cost_usd: None,
            cost_status: CostStatus::Missing,
            quality: UsageDataQuality::Parsed,
            event_count: 1,
        };
        let diagnostics = vec![UsageScanDiagnostic {
            client: Some("codex".to_string()),
            source_kind: Some("tokscale-local".to_string()),
            code: "scan_failed".to_string(),
            message: "local usage scan failed".to_string(),
        }];
        let mut report = test_usage_report(vec![summary], diagnostics, false);
        report.coverage = UsageCoverage {
            requested_clients: vec!["codex".to_string()],
            available_clients: vec!["codex".to_string()],
            missing_clients: Vec::new(),
            status: "partial".to_string(),
        };
        report.freshness.stale = true;

        let payload = usage_json_payload(&report);

        assert_eq!(payload["coverage"]["status"], "partial");
        assert_eq!(payload["clients"][0]["client"], "codex");
        assert_eq!(payload["diagnostics"][0]["code"], "scan_failed");
    }

    fn test_usage_report(
        groups: Vec<UsageSummary>,
        diagnostics: Vec<UsageScanDiagnostic>,
        no_scan: bool,
    ) -> UsageReport {
        UsageReport {
            query: UsageQuery {
                client: None,
                since_unix: Some(1_782_144_000),
                until_unix: Some(1_782_230_400),
                period: UsagePeriod::Custom,
                group_by: UsageGroupBy::Client,
                details: false,
            },
            totals: usage_total(&groups),
            groups,
            freshness: UsageFreshness {
                generated_at_unix: 1,
                last_scan_at_unix: (!no_scan).then_some(1),
                stale: no_scan,
            },
            coverage: UsageCoverage {
                requested_clients: Vec::new(),
                available_clients: Vec::new(),
                missing_clients: Vec::new(),
                status: "empty".to_string(),
            },
            accounting: UsageAccounting {
                quality: UsageDataQuality::Parsed,
                cost_status: CostStatus::Missing,
                note: usage_accounting_note().to_string(),
            },
            diagnostics,
            scan: UsageReportScan {
                enabled: !no_scan,
                scanned_events: 0,
                inserted_events: 0,
            },
        }
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
        };

        assert_eq!(
            resolve_target(&plugin, "1").unwrap(),
            omx_core::TargetResolution {
                kind: TargetKind::Account,
                target_id: "claude-account-1".to_string()
            }
        );
        assert_eq!(
            resolve_target(&plugin, "3").unwrap(),
            omx_core::TargetResolution {
                kind: TargetKind::Profile,
                target_id: "claude-profile-gateway".to_string()
            }
        );
        assert_eq!(
            resolve_target(&plugin, "gateway").unwrap(),
            omx_core::TargetResolution {
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
        };

        assert_eq!(
            resolve_target(&plugin, "4").unwrap(),
            omx_core::TargetResolution {
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
                diagnostics: Vec::new(),
            }),
        }
    }

    fn account_status_with_alias(number: u32, alias: Option<&str>) -> AccountStatus {
        AccountStatus {
            account: AccountRef {
                platform: "claude".to_string(),
                local_id: format!("claude-account-{number}"),
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
            local_id: format!("claude-profile-{name}"),
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
