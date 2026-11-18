// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::Value;

use crate::error::{Code, Diagnostic, Error, Result};

mod cache;
mod install;
mod lock;
mod support;

use cache::{decode_path, read_environment, write_environment};
use lock::LOCK_WAIT;
use support::{Version, python_supported};

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
        std::env::join_paths(entries).map_err(|err| -> Error {
            failure(
                "construct the python subprocess PATH",
                err.to_string(),
                "keep uv's parent directory and PATH as valid platform paths",
            )
            .into()
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
    wait: Duration,
}

impl<'a> Request<'a> {
    fn new(root: PathBuf, release: &'a str, now: u64) -> Request<'a> {
        Request {
            paths: Paths::new(root),
            release,
            now,
            wait: LOCK_WAIT,
        }
    }
}

pub fn ensure(now: u64) -> Result<Environment> {
    let root = cache::state_root()?;
    ensure_with(
        Request::new(root, env!("CARGO_PKG_VERSION"), now),
        probe_with,
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
    mut probe: impl FnMut(&Paths) -> Result<Observed>,
    mut repair: impl FnMut(Repair, &Paths, Observed) -> Result<Environment>,
) -> Result<Environment> {
    let Request {
        paths,
        release,
        now,
        wait,
    } = request;
    if let Some(environment) = read_environment(&paths, release)? {
        return Ok(environment);
    }
    let _lock = lock::Lock::acquire(&paths, wait)?;
    if let Some(environment) = read_environment(&paths, release)? {
        return Ok(environment);
    }
    let observed = probe(&paths)?;
    let environment = match plan(&observed, now) {
        Some(kind) => repair(kind, &paths, observed)?,
        None => match (observed.python, observed.uv) {
            (Some(python), Some(uv)) => Environment::new(python, uv),
            (python, uv) => {
                return Err(failure(
                    "assemble the observed environment",
                    "the no-repair decision did not carry both programs",
                    "retry the environment check and keep Python and uv available",
                )
                .value(format!(
                    "python present: {}, uv present: {}",
                    python.is_some(),
                    uv.is_some()
                )));
            }
        },
    };
    if !python_supported(&environment.python.version, now) {
        return Err(failure(
            "validate the repaired Python before publishing the cache",
            "the repaired interpreter is outside the supported Python 3 lifetime",
            format!(
                "remove the unsupported private installation and retry: {}",
                crate::error::remove_tree_instruction(&paths.python)
            ),
        )
        .at_with_value(&environment.python.executable, &environment.python.version));
    }
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

fn probe_with(paths: &Paths) -> Result<Observed> {
    Ok(Observed {
        python: probe_python_from_path()?,
        uv: match probe_uv_from_path()? {
            Some(uv) => Some(uv),
            None => probe_private_uv(paths)?,
        },
    })
}

fn repair(kind: Repair, paths: &Paths, observed: Observed) -> Result<Environment> {
    let deadline = install::Deadline::start()?;
    match kind {
        Repair::Uv => {
            let uv = install::uv(paths, &deadline)?;
            let python = observed.python.ok_or_else(|| {
                failure(
                    "assemble the uv-only repair",
                    "the selected Python is missing",
                    "retry the environment check",
                )
            })?;
            Ok(Environment::new(python, uv))
        }
        Repair::Python => {
            let uv = install::uv(paths, &deadline)?;
            let python = install::python(paths, &uv, &deadline)?;
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
    let output = match Command::new(program).args(["-I", "-c", script]).output() {
        Ok(output) if output.status.success() => output,
        Ok(_) => return Ok(None),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(match probe_path(program) {
                Some(path) => failure(
                    "probe the Python interpreter",
                    err.to_string(),
                    format!(
                        "make the interpreter runnable, or remove it from the probe path: {}",
                        crate::error::remove_file_instruction(path)
                    ),
                )
                .at_with_value(path, program.to_string_lossy()),
                None => failure(
                    "probe the Python interpreter",
                    err.to_string(),
                    "fix PATH so the intended Python interpreter is runnable; no unlocated file is deleted",
                )
                .value(program.to_string_lossy()),
            });
        }
    };
    let value: Value = match serde_json::from_slice(&output.stdout) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let version = value.get("version").and_then(Value::as_str);
    let executable = value.get("executable").and_then(decode_path);
    Ok(match (version, executable) {
        (Some(version), Some(executable)) if Version::parse(version).is_some() => {
            Some(Program::new(version, executable))
        }
        _ => None,
    })
}

fn probe_path(program: &OsStr) -> Option<&Path> {
    let path = Path::new(program);
    path.has_root().then_some(path)
}

fn probe_python_from_path() -> Result<Option<Program>> {
    for candidate in executable_candidates(python_program(), std::env::var_os("PATH"))? {
        if let Some(program) = probe_python(candidate.as_os_str())? {
            return Ok(Some(program));
        }
    }
    Ok(None)
}

fn probe_uv_from_path() -> Result<Option<Program>> {
    probe_uv_search(std::env::var_os("PATH"))
}

fn probe_uv_search(path: Option<OsString>) -> Result<Option<Program>> {
    for candidate in executable_candidates(uv_program(), path)? {
        if let Some(program) = probe_uv(&candidate)? {
            return Ok(Some(program));
        }
    }
    Ok(None)
}

fn executable_candidates(program: &str, path: Option<OsString>) -> Result<Vec<PathBuf>> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let mut candidates = Vec::new();
    for directory in std::env::split_paths(&path) {
        let directory = if directory.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            directory
        };
        if cfg!(windows) && Path::new(program).extension().is_none() {
            let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE".to_owned());
            for extension in extensions
                .split(';')
                .filter(|extension| !extension.is_empty())
            {
                let candidate = directory.join(format!("{program}{extension}"));
                if is_executable_file(&candidate)? {
                    candidates.push(candidate);
                }
            }
        } else {
            let candidate = directory.join(program);
            if is_executable_file(&candidate)? {
                candidates.push(candidate);
            }
        }
    }
    Ok(candidates)
}

fn is_executable_file(path: &Path) -> Result<bool> {
    let metadata = match path.metadata() {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => {
            return Err(failure(
                "inspect a runtime candidate",
                err.to_string(),
                "keep the runtime candidate readable or remove it from PATH",
            )
            .at(path));
        }
    };
    #[cfg(unix)]
    let executable = {
        use std::os::unix::fs::PermissionsExt as _;
        metadata.permissions().mode() & 0o111 != 0
    };
    #[cfg(windows)]
    let executable = true;
    Ok(metadata.is_file() && executable)
}

fn probe_private_uv(paths: &Paths) -> Result<Option<Program>> {
    let candidate = paths.bin.join(uv_program());
    if !is_executable_file(&candidate)? {
        return Ok(None);
    }
    probe_uv(&candidate)
}

fn uv_program() -> &'static str {
    if cfg!(windows) { "uv.exe" } else { "uv" }
}

#[expect(
    clippy::disallowed_methods,
    reason = "Mei probes its own uv installation outside city child management"
)]
fn probe_uv(path: &Path) -> Result<Option<Program>> {
    let output = match Command::new(path).arg("--version").output() {
        Ok(output) if output.status.success() => output,
        Ok(_) => return Ok(None),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(failure(
                "probe uv",
                err.to_string(),
                format!(
                    "make uv runnable or remove it: {}",
                    crate::error::remove_file_instruction(path)
                ),
            )
            .at(path));
        }
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let version = text.split_whitespace().nth(1);
    let executable = match std::path::absolute(path) {
        Ok(path) => path,
        Err(err) => {
            return Err(failure(
                "resolve the uv executable path",
                err.to_string(),
                "keep the current directory readable, or name uv by an absolute path",
            )
            .at(path));
        }
    };
    Ok(version.and_then(|version| {
        Version::parse(version).map(|_| Program::new(version, executable.clone()))
    }))
}

fn failure(
    action: impl Into<String>,
    subject: impl Into<String>,
    recovery: impl Into<String>,
) -> Failure {
    Failure(Diagnostic::new(Code::Doctor, action, subject, recovery))
}

struct Failure(Diagnostic);

impl Failure {
    fn at(self, path: &Path) -> Error {
        Error::Diagnostic(self.0.with_path(path.display().to_string()))
    }

    fn value(self, value: impl Into<String>) -> Error {
        Error::Diagnostic(self.0.with_value(value))
    }

    fn at_with_value(self, path: &Path, value: impl Into<String>) -> Error {
        Error::Diagnostic(
            self.0
                .with_path(path.display().to_string())
                .with_value(value),
        )
    }
}

impl From<Failure> for Error {
    fn from(failure: Failure) -> Error {
        Error::Diagnostic(failure.0)
    }
}

#[cfg(test)]
#[path = "doctor_tests.rs"]
mod tests;
