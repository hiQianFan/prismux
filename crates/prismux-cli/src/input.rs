use anyhow::{Context, Result, bail};
use std::{
    io::{self, IsTerminal, Read},
    path::Path,
    process::Command as ProcessCommand,
};

pub fn read_import_content(
    file: Option<&Path>,
    clipboard: bool,
    content: Vec<String>,
) -> Result<String> {
    read_optional_import_content(file, clipboard, content)?.ok_or_else(|| {
        anyhow::anyhow!(
            "missing config content. Paste TOML/KV at the end, pass --file <path>, use @<path>, --clipboard, or pipe through stdin"
        )
    })
}

pub fn read_optional_import_content(
    file: Option<&Path>,
    clipboard: bool,
    content: Vec<String>,
) -> Result<Option<String>> {
    if let Some(path) = file {
        return std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))
            .map(Some);
    }

    if clipboard {
        return read_clipboard_content().map(Some);
    }

    if !content.is_empty() {
        if content.len() == 1 {
            let value = &content[0];
            if let Some(path) = value.strip_prefix('@') {
                return std::fs::read_to_string(path)
                    .with_context(|| format!("failed to read config file {path}"))
                    .map(Some);
            }
            let path = Path::new(value);
            if path.exists() && path.is_file() {
                return std::fs::read_to_string(path)
                    .with_context(|| format!("failed to read config file {}", path.display()))
                    .map(Some);
            }
        }
        return Ok(Some(content.join(" ")));
    }

    if !io::stdin().is_terminal() {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        if !input.trim().is_empty() {
            return Ok(Some(input));
        }
    }

    Ok(None)
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
