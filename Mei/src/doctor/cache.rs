// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{Environment, Paths, Program, failure};
use crate::error::Result;

const CACHE_SCHEMA: u64 = 1;

pub(super) fn read_environment(paths: &Paths, release: &str) -> Result<Option<Environment>> {
    let bytes = match std::fs::read(&paths.cache) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(failure(
                "read the doctor cache",
                err.to_string(),
                format!(
                    "keep the cache readable, or remove it to rebuild the environment: {}",
                    crate::error::remove_file_command(&paths.cache)
                ),
            )
            .at(&paths.cache));
        }
    };
    let value: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    if value.get("schema").and_then(Value::as_u64) != Some(CACHE_SCHEMA)
        || value.get("mei").and_then(Value::as_str) != Some(release)
    {
        return Ok(None);
    }
    Ok(match (program(&value, "python"), program(&value, "uv")) {
        (Some(python), Some(uv)) => Some(Environment::new(python, uv)),
        _ => None,
    })
}

fn program(value: &Value, key: &str) -> Option<Program> {
    let item = value.get(key)?;
    let version = item.get("version")?.as_str()?;
    let executable = decode_path(item.get("executable")?)?;
    if super::Version::parse(version).is_none() || !executable.is_file() {
        return None;
    }
    Some(Program::new(version, executable))
}

pub(super) fn write_environment(
    paths: &Paths,
    release: &str,
    environment: &Environment,
) -> Result<()> {
    std::fs::create_dir_all(&paths.root).map_err(|err| {
        failure(
            "create the doctor directory",
            err.to_string(),
            "keep the per-user doctor directory writable",
        )
        .at(&paths.root)
    })?;
    let value = json!({
        "schema": CACHE_SCHEMA,
        "mei": release,
        "python": program_json(&environment.python),
        "uv": program_json(&environment.uv),
    });
    let bytes = serde_json::to_vec(&value).map_err(|err| {
        failure(
            "serialize the doctor cache",
            err.to_string(),
            "keep environment versions and paths serializable",
        )
        .at(&paths.cache)
    })?;
    let temporary = paths
        .root
        .join(format!("environment-{}.json", std::process::id()));
    std::fs::write(&temporary, bytes).map_err(|err| {
        failure(
            "write the doctor cache candidate",
            err.to_string(),
            format!(
                "keep the doctor directory writable, or remove the candidate: {}",
                crate::error::remove_file_command(&temporary)
            ),
        )
        .at(&temporary)
    })?;
    // `rename` replaces the target on every platform this tree builds for, so
    // concurrent writers of the same record converge instead of leaving the
    // cache absent for a reader that is mid-probe.
    std::fs::rename(&temporary, &paths.cache).map_err(|err| {
        failure(
            "publish the doctor cache",
            err.to_string(),
            "keep the cache directory writable and retry the environment check",
        )
        .at(&paths.cache)
    })
}

fn program_json(program: &Program) -> Value {
    json!({"version": program.version, "executable": encode_path(&program.executable)})
}

#[cfg(unix)]
pub(super) fn encode_path(path: &Path) -> Value {
    use std::os::unix::ffi::OsStrExt as _;
    json!(path.as_os_str().as_bytes().to_vec())
}

#[cfg(windows)]
pub(super) fn encode_path(path: &Path) -> Value {
    use std::os::windows::ffi::OsStrExt as _;
    let bytes = path
        .as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    json!(bytes)
}

#[cfg(unix)]
pub(super) fn decode_path(value: &Value) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStringExt as _;
    let bytes = value
        .as_array()?
        .iter()
        .map(|byte| u8::try_from(byte.as_u64()?).ok())
        .collect::<Option<Vec<_>>>()?;
    Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}

#[cfg(windows)]
pub(super) fn decode_path(value: &Value) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt as _;
    let bytes = value
        .as_array()?
        .iter()
        .map(|byte| u8::try_from(byte.as_u64()?).ok())
        .collect::<Option<Vec<_>>>()?;
    let (pairs, remainder) = bytes.as_chunks::<2>();
    if !remainder.is_empty() {
        return None;
    }
    let wide = pairs
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect::<Vec<_>>();
    Some(PathBuf::from(std::ffi::OsString::from_wide(&wide)))
}
