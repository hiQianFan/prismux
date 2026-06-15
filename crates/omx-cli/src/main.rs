use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use omx_core::PlatformPlugin;
use omx_plugin_codex::CodexPlugin;

#[derive(Debug, Parser)]
#[command(name = "omx", version, about = "OpenMux CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List stored accounts.
    List,
    /// Show the current account for a platform.
    Current {
        /// Platform id, for example: codex.
        platform: String,
    },
    /// Show status for known platforms.
    Status,
    /// Import the currently active account.
    Import {
        /// Platform id, for example: codex.
        platform: String,
        /// Local account alias.
        alias: String,
    },
    /// Switch the active account.
    Use {
        /// Platform id, for example: codex.
        platform: String,
        /// Local account alias.
        alias: String,
    },
    /// Run platform diagnostics.
    Doctor,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let plugins: Vec<Box<dyn PlatformPlugin>> = vec![Box::new(CodexPlugin::new())];

    match cli.command {
        Command::List => {
            for plugin in &plugins {
                let accounts = plugin.list_accounts()?;
                println!("{} ({})", plugin.name(), plugin.id());
                if accounts.is_empty() {
                    println!("  no accounts imported");
                } else {
                    for account in accounts {
                        let marker = if account.active { "*" } else { " " };
                        println!("{marker} {}", account.account.alias);
                    }
                }
            }
        }
        Command::Current { platform } => {
            let plugin = find_plugin(&plugins, &platform)?;
            match plugin.current()? {
                Some(status) => println!("{}", status.account.alias),
                None => println!("no active account"),
            }
        }
        Command::Status => {
            for plugin in &plugins {
                let install = plugin.detect()?;
                println!("{} ({})", install.platform.name, install.platform.id);
                println!(
                    "  config: {}",
                    optional_path(install.config_path.as_deref())
                );
                println!("  auth: {}", optional_path(install.auth_path.as_deref()));
            }
        }
        Command::Import { platform, alias } => {
            let plugin = find_plugin(&plugins, &platform)?;
            let account = plugin.import_current(&alias)?;
            println!("imported {}:{}", account.platform, account.alias);
        }
        Command::Use { platform, alias } => {
            let plugin = find_plugin(&plugins, &platform)?;
            let report = plugin.switch_to(&alias)?;
            println!("using {}:{}", report.current.platform, report.current.alias);
        }
        Command::Doctor => {
            for plugin in &plugins {
                let report = plugin.doctor()?;
                println!("{}", report.platform);
                for check in report.checks {
                    let marker = if check.ok { "ok" } else { "fail" };
                    println!("  [{marker}] {} - {}", check.name, check.message);
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

fn optional_path(path: Option<&str>) -> &str {
    path.unwrap_or("not detected yet")
}
