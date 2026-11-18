// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::error::{Code, Diagnostic, Error, Result};

const LOCK_POLLS: u16 = 300;
const LOCK_POLL: std::time::Duration = std::time::Duration::from_millis(100);

mod cache;
mod install;

use cache::{decode_path, read_environment, write_environment};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub version: String,
    pub executable: PathBuf,
}

impl Program {
    pub fn new(version: impl Into<String>, executable: PathBuf) -> Program {
        Program {
            version: version.into(),
            executable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    pub python: Program,
    pub uv: Program,
}

impl Environment {
    pub fn new(python: Program, uv: Program) -> Environment {
        Environment { python, uv }
    }

    pub fn child_path(&self) -> Result<OsString> {
        let mut entries = Vec::new();
        if let Some(parent) = self.uv.executable.parent() {
            entries.push(parent.to_path_buf());
        }
        if let Some(path) = std::env::var_os("PATH") {
            entries.extend(std::env::split_paths(&path));
        }
        std::env::join_paths(entries).map_err(|err| {
            failure(
                "construct the python subprocess PATH",
                err.to_string(),
                "keep uv's parent directory and PATH as valid platform paths",
                None,
            )
        })
    }
}

#[derive(Debug, Clone)]
struct Observed {
    python: Option<Program>,
    uv: Option<Program>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Repair {
    Uv,
    Python,
}

#[derive(Debug, Clone)]
struct Paths {
    root: PathBuf,
    cache: PathBuf,
    lock: PathBuf,
    bin: PathBuf,
    python: PathBuf,
    python_bin: PathBuf,
    uv_cache: PathBuf,
}

impl Paths {
    fn new(root: PathBuf) -> Paths {
        Paths {
            cache: root.join("environment.json"),
            lock: root.join("environment.lock"),
            bin: root.join("bin"),
            python: root.join("python"),
            python_bin: root.join("python-bin"),
            uv_cache: root.join("uv-cache"),
            root,
        }
    }
}

struct Request<'a> {
    paths: Paths,
    release: &'a str,
    now: u64,
}

impl<'a> Request<'a> {
    fn new(root: PathBuf, release: &'a str, now: u64) -> Request<'a> {
        Request {
            paths: Paths::new(root),
            release,
            now,
        }
    }
}

pub fn ensure(now: u64) -> Result<Environment> {
    let root = state_root()?;
    ensure_with(
        Request::new(root, env!("CARGO_PKG_VERSION"), now),
        probe,
        repair,
    )
}

pub fn write_cache(root: &Path, environment: &Environment) -> Result<()> {
    write_environment(
        &Paths::new(root.to_path_buf()),
        env!("CARGO_PKG_VERSION"),
        environment,
    )
}

fn ensure_with(
    request: Request,
    mut probe: impl FnMut() -> Result<Observed>,
    mut repair: impl FnMut(Repair, &Paths, Observed) -> Result<Environment>,
) -> Result<Environment> {
    let Request {
        paths,
        release,
        now,
    } = request;
    if let Some(environment) = read_environment(&paths, release)? {
        return Ok(environment);
    }
    let _lock = Lock::acquire(&paths)?;
    if let Some(environment) = read_environment(&paths, release)? {
        return Ok(environment);
    }
    let observed = probe()?;
    let environment = match plan(&observed, now) {
        Some(kind) => repair(kind, &paths, observed)?,
        None => match (observed.python, observed.uv) {
            (Some(python), Some(uv)) => Environment::new(python, uv),
            _ => {
                return Err(failure(
                    "assemble the observed environment",
                    "the no-repair decision did not carry both programs",
                    "retry the environment check and keep Python and uv available",
                    None,
                ));
            }
        },
    };
    write_environment(&paths, release, &environment)?;
    Ok(environment)
}

fn plan(observed: &Observed, now: u64) -> Option<Repair> {
    let python_is_supported = observed
        .python
        .as_ref()
        .is_some_and(|python| python_supported(&python.version, now));
    if !python_is_supported {
        Some(Repair::Python)
    } else if observed.uv.is_none() {
        Some(Repair::Uv)
    } else {
        None
    }
}

fn probe() -> Result<Observed> {
    Ok(Observed {
        python: probe_python(OsStr::new(python_program()))?,
        uv: probe_uv_from_path()?,
    })
}

fn repair(kind: Repair, paths: &Paths, observed: Observed) -> Result<Environment> {
    match kind {
        Repair::Uv => {
            let uv = install::uv(paths)?;
            let python = observed.python.ok_or_else(|| {
                failure(
                    "assemble the uv-only repair",
                    "the selected Python is missing",
                    "retry the environment check",
                    None,
                )
            })?;
            Ok(Environment::new(python, uv))
        }
        Repair::Python => {
            let uv = install::uv(paths)?;
            let python = install::python(paths, &uv)?;
            Ok(Environment::new(python, uv))
        }
    }
}

fn python_program() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
}

#[expect(
    clippy::disallowed_methods,
    reason = "Mei probes its own interpreter outside city child management"
)]
fn probe_python(program: &OsStr) -> Result<Option<Program>> {
    let script = concat!(
        "import json, os, platform, sys; ",
        "raw = sys.executable.encode('utf-16-le') if os.name == 'nt' else os.fsencode(sys.executable); ",
        "print(json.dumps({'version': platform.python_version(), 'executable': list(raw)}))"
    );
    let output = match Command::new(program).args(["-c", script]).output() {
        Ok(output) if output.status.success() => output,
        Ok(_) | Err(_) => return Ok(None),
    };
    let value: Value = match serde_json::from_slice(&output.stdout) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let version = value.get("version").and_then(Value::as_str);
    let executable = value.get("executable").and_then(decode_path);
    Ok(match (version, executable) {
        (Some(version), Some(executable)) if valid_version(version) => {
            Some(Program::new(version, executable))
        }
        _ => None,
    })
}

fn probe_uv_from_path() -> Result<Option<Program>> {
    let Some(path) = std::env::var_os("PATH") else {
        return Ok(None);
    };
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join(if cfg!(windows) { "uv.exe" } else { "uv" });
        if !candidate.is_file() {
            continue;
        }
        if let Some(program) = probe_uv(&candidate)? {
            return Ok(Some(program));
        }
    }
    Ok(None)
}

#[expect(
    clippy::disallowed_methods,
    reason = "Mei probes its own uv installation outside city child management"
)]
fn probe_uv(path: &Path) -> Result<Option<Program>> {
    let output = match Command::new(path).arg("--version").output() {
        Ok(output) if output.status.success() => output,
        Ok(_) | Err(_) => return Ok(None),
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let version = text.split_whitespace().nth(1);
    let executable = match std::path::absolute(path) {
        Ok(path) => path,
        Err(_) => return Ok(None),
    };
    Ok(version.and_then(|version| {
        valid_version(version).then(|| Program::new(version, executable.clone()))
    }))
}

fn valid_version(version: &str) -> bool {
    version
        .split('.')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        && version.contains('.')
}

fn python_supported(version: &str, now: u64) -> bool {
    let mut parts = version.split('.');
    let (Some(major), Some(minor)) = (parts.next(), parts.next()) else {
        return false;
    };
    let (Ok(major), Ok(minor)) = (major.parse::<u32>(), minor.parse::<u32>()) else {
        return false;
    };
    if major != 3 {
        return false;
    }
    let end_of_life = match minor {
        0 => 1_246_147_200,
        1 => 1_334_016_000,
        2 => 1_383_091_200,
        3 => 1_506_729_600,
        4 => 1_552_953_600,
        5 => 1_601_510_400,
        6 => 1_640_304_000,
        7 => 1_687_910_400,
        8 => 1_728_345_600,
        9 => 1_761_955_200,
        10 => 1_790_812_800,
        11 => 1_825_027_200,
        12 => 1_856_649_600,
        13 => 1_888_185_600,
        14 => 1_919_721_600,
        15 => 1_951_257_600,
        16 => 1_982_880_000,
        _ => return true,
    };
    now < end_of_life
}

struct Lock {
    path: PathBuf,
}

impl Lock {
    fn acquire(paths: &Paths) -> Result<Lock> {
        std::fs::create_dir_all(&paths.root).map_err(|err| {
            failure(
                "create the doctor lock directory",
                err.to_string(),
                "keep the per-user doctor directory writable",
                Some(&paths.root),
            )
        })?;
        for _ in 0..LOCK_POLLS {
            match std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&paths.lock)
            {
                Ok(_) => {
                    return Ok(Lock {
                        path: paths.lock.clone(),
                    });
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    std::thread::sleep(LOCK_POLL);
                }
                Err(err) => {
                    return Err(failure(
                        "acquire the doctor lock",
                        err.to_string(),
                        "keep the doctor directory writable",
                        Some(&paths.lock),
                    ));
                }
            }
        }
        Err(failure(
            "wait for another Mei environment check",
            "the doctor lock did not clear within 30 seconds",
            "if no other Mei is installing Python or uv, remove the stale lock and retry",
            Some(&paths.lock),
        ))
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_file(&self.path) {
            eprintln!("remove doctor lock {}: {err}", self.path.display());
        }
    }
}

fn state_root() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("MEI_DOCTOR_DIR") {
        if path.is_empty() {
            return Err(failure(
                "read MEI_DOCTOR_DIR",
                "the directory is empty",
                "leave MEI_DOCTOR_DIR unset or provide a writable directory",
                None,
            ));
        }
        return Ok(PathBuf::from(path));
    }
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")));
    base.map(|base| base.join("sprawling").join("mei"))
        .ok_or_else(|| {
            failure(
                "locate the doctor directory",
                "neither the platform cache directory nor a home directory is available",
                "set MEI_DOCTOR_DIR to a writable per-user directory",
                None,
            )
        })
}

fn failure(
    action: impl Into<String>,
    subject: impl Into<String>,
    recovery: impl Into<String>,
    path: Option<&Path>,
) -> Error {
    let diagnostic = Diagnostic::new(Code::Doctor, action, subject, recovery);
    Error::Diagnostic(match path {
        Some(path) => diagnostic.with_path(path.display().to_string()),
        None => diagnostic,
    })
}

#[cfg(test)]
#[path = "doctor_tests.rs"]
mod tests;
