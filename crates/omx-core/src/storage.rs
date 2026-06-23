use crate::{OpenMuxError, Result};
use sha2::{Digest, Sha256};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn create_dir_private(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|err| io_error(path, err))?;
    set_private_dir_permissions(path)
}

pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|err| io_error(path, err))
}

pub fn write_file_atomic_private(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_private(parent)?;
    }

    let tmp_path = path.with_extension(format!("tmp.{}.{}", std::process::id(), unix_now_nanos()));
    write_private_file(&tmp_path, bytes)?;
    fs::rename(&tmp_path, path).map_err(|err| {
        let _ = fs::remove_file(&tmp_path);
        io_error(path, err)
    })?;
    set_private_file_permissions(path)
}

pub fn prune_backup_files(dir: &Path, prefix: &str, keep_latest: usize) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(io_error(dir, err)),
    };
    let mut backups = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| io_error(dir, err))?;
        let file_type = entry
            .file_type()
            .map_err(|err| io_error(&entry.path(), err))?;
        if !file_type.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        let Some(timestamp) = file_name
            .strip_prefix(prefix)
            .and_then(|suffix| suffix.parse::<u128>().ok())
        else {
            continue;
        };
        backups.push((timestamp, entry.path()));
    }

    backups.sort_by_key(|backup| std::cmp::Reverse(backup.0));
    for (_timestamp, path) in backups.into_iter().skip(keep_latest) {
        fs::remove_file(&path).map_err(|err| io_error(&path, err))?;
    }
    Ok(())
}

#[cfg(unix)]
fn write_private_file(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};

    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(|err| io_error(path, err))?;
    file.write_all(bytes).map_err(|err| io_error(path, err))
}

#[cfg(not(unix))]
fn write_private_file(path: &Path, bytes: &[u8]) -> Result<()> {
    fs::write(path, bytes).map_err(|err| io_error(path, err))?;
    set_private_file_permissions(path)
}

#[cfg(unix)]
pub fn set_private_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|err| io_error(path, err))
}

#[cfg(not(unix))]
pub fn set_private_file_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
pub fn set_private_dir_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|err| io_error(path, err))
}

#[cfg(not(unix))]
pub fn set_private_dir_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

pub fn unix_now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("USERPROFILE")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
}

pub fn data_local_dir() -> Option<PathBuf> {
    if let Some(path) = env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(path));
    }

    #[cfg(target_os = "macos")]
    {
        home_dir().map(|path| path.join("Library").join("Application Support"))
    }

    #[cfg(target_os = "windows")]
    {
        env::var_os("LOCALAPPDATA")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        home_dir().map(|path| path.join(".local").join("share"))
    }
}

pub fn state_root() -> Result<PathBuf> {
    if let Some(path) = env::var_os("OMUX_STATE_ROOT").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    data_local_dir()
        .map(|path| path.join("openmux"))
        .ok_or_else(|| OpenMuxError::Message("could not resolve the OpenMux data directory".into()))
}

pub fn io_error(path: &Path, err: io::Error) -> OpenMuxError {
    OpenMuxError::Message(format!("{}: {err}", display_path(path)))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_private_write_replaces_file_without_leaving_temp_file() {
        let root = env::temp_dir().join(format!(
            "openmux-storage-test-{}-{}",
            std::process::id(),
            unix_now_nanos()
        ));
        let path = root.join("auth.json");

        write_file_atomic_private(&path, b"old").unwrap();
        write_file_atomic_private(&path, b"new").unwrap();

        assert_eq!(read_file(&path).unwrap(), b"new");
        let entries = fs::read_dir(&root)
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path(), path);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = fs::metadata(entries[0].path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prune_backup_files_keeps_latest_matching_files() {
        let root = env::temp_dir().join(format!(
            "openmux-storage-prune-test-{}-{}",
            std::process::id(),
            unix_now_nanos()
        ));
        create_dir_private(&root).unwrap();
        for timestamp in [10, 30, 20, 40] {
            write_file_atomic_private(
                &root.join(format!("auth.json.bak.{timestamp}")),
                timestamp.to_string().as_bytes(),
            )
            .unwrap();
        }
        write_file_atomic_private(&root.join("config.toml.bak.50"), b"keep").unwrap();

        prune_backup_files(&root, "auth.json.bak.", 2).unwrap();

        let mut files = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        files.sort();
        assert_eq!(
            files,
            vec![
                "auth.json.bak.30".to_string(),
                "auth.json.bak.40".to_string(),
                "config.toml.bak.50".to_string(),
            ]
        );

        let _ = fs::remove_dir_all(root);
    }
}
