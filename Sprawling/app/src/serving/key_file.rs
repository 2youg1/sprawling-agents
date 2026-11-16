// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write as _;
use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError};

const DIR: &str = "sprawling";

#[must_use = "a key file left behind outlives the city it opens"]
pub(crate) struct KeyFile {
    path: PathBuf,
}

impl KeyFile {

    pub(crate) fn write(port: u16, key: &str) -> Result<Self, AxError> {
        let dir = runtime_dir()?;
        let path = dir.join(format!("{port}.key"));
        let staged = dir.join(format!("{port}.key.{}", std::process::id()));
        let unwritten =
            |source: std::io::Error| refused("write the city's key file", &staged, &source);

        match std::fs::remove_file(&staged) {
            Ok(()) => {}
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => return Err(unwritten(source)),
        }
        let mut file = private_file(&staged).map_err(unwritten)?;
        file.write_all(key.as_bytes()).map_err(unwritten)?;
        file.sync_all().map_err(unwritten)?;
        drop(file);
        std::fs::rename(&staged, &path)
            .map_err(|source| refused("write the city's key file", &path, &source))?;
        Ok(Self { path })
    }

    pub(crate) fn remove(self) -> Result<(), AxError> {
        std::fs::remove_file(&self.path)
            .map_err(|source| refused("remove the city's key file", &self.path, &source))
    }
}

pub fn read_key(port: u16) -> Result<Option<String>, AxError> {
    let path = platform_dir()?.join(DIR).join(format!("{port}.key"));
    match std::fs::read_to_string(&path) {
        Ok(key) => Ok(Some(key.trim().to_owned())),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(refused("read the city's key file", &path, &source)),
    }
}

pub(crate) fn runtime_dir() -> Result<PathBuf, AxError> {
    let dir = platform_dir()?.join(DIR);
    std::fs::create_dir_all(&dir)
        .map_err(|source| refused("make the runtime directory", &dir, &source))?;
    make_private(&dir)?;
    Ok(dir)
}

#[cfg(windows)]
fn platform_dir() -> Result<PathBuf, AxError> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| unnamed("LOCALAPPDATA"))
}

#[cfg(target_os = "macos")]
fn platform_dir() -> Result<PathBuf, AxError> {
    std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .ok_or_else(|| unnamed("TMPDIR"))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn platform_dir() -> Result<PathBuf, AxError> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .ok_or_else(|| unnamed("XDG_RUNTIME_DIR"))
}

#[cfg(unix)]
fn make_private(dir: &Path) -> Result<(), AxError> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
        .map_err(|source| refused("make the runtime directory private", dir, &source))
}

#[cfg(windows)]
fn make_private(dir: &Path) -> Result<(), AxError> {
    let user = match (std::env::var("USERDOMAIN"), std::env::var("USERNAME")) {
        (Ok(domain), Ok(name)) => format!("{domain}\\{name}"),
        (Err(_), Ok(name)) => name,
        (_, Err(_)) => return Err(unnamed("USERNAME")),
    };
    let status = child::command(crate::privacy::windows::system32("icacls.exe")?)
        .arg(dir)
        .args(["/inheritance:r", "/grant:r"])
        .arg(format!("{user}:(OI)(CI)F"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|source| refused("make the runtime directory private", dir, &source))?;
    if status.success() {
        return Ok(());
    }
    Err(AxError::failure(
        AxCode::StorageFatal,
        "make the runtime directory private",
        format!("icacls answered {status} for {}", dir.display()),
    )
    .with_recovery("check that this account may change the permissions of its own LOCALAPPDATA"))
}

#[cfg(unix)]
fn private_file(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(windows)]
fn private_file(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
}

fn refused(action: &'static str, path: &Path, source: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        action,
        format!("{}: {source}", path.display()),
    )
    .with_recovery("check that the per-user runtime directory exists and belongs to this account")
}

fn unnamed(variable: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "find the per-user runtime directory",
        format!("{variable} is not set"),
    )
    .with_recovery(format!(
        "set {variable} to a directory only this account can read"
    ))
}
