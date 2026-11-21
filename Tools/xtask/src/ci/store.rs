// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Success-only verdict storage. The local disk and GitHub Actions cache
//! adapters share one validation rule (xtask `spec/Ci.lean`).

use std::io::Write as _;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::report::XtaskError;

use super::content::{Environment, Key};
use super::diagnostic::Diagnostic;
use super::graph::Reuse;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Verdict {
    Green,
    Refused,
    Broken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Cached {
    pub(crate) key: String,
    pub(crate) verdict: Verdict,
}

/// Lean `may_reuse`: a green result under the same complete content key.
pub(crate) fn may_reuse(policy: Reuse, cached: &Cached, current: &Key) -> bool {
    policy == Reuse::SuccessOnly
        && cached.verdict == Verdict::Green
        && cached.key == current.as_str()
}

#[derive(Debug)]
pub(crate) struct RemoteStore {
    adapter: PathBuf,
    work: PathBuf,
}

#[derive(Debug)]
pub(crate) enum Store {
    Local(PathBuf),
    Remote(RemoteStore),
}

impl Store {
    #[cfg(test)]
    pub(crate) fn local(path: PathBuf) -> Self {
        Self::Local(path)
    }

    pub(crate) fn for_root(
        root: &Path,
        local_override: Option<PathBuf>,
    ) -> Result<Self, XtaskError> {
        if let Some(path) = local_override {
            return output_path(root, &path).map(Self::Local);
        }
        Ok(match std::env::var_os("SPRAWLING_CI_CACHE_ADAPTER") {
            Some(adapter) if !adapter.is_empty() => Self::Remote(RemoteStore {
                adapter: root.join(adapter),
                work: output_path(root, Path::new("target/ci/remote"))?,
            }),
            _ => Self::Local(output_path(root, Path::new("target/ci/verdicts"))?),
        })
    }

    pub(crate) fn restore(
        &self,
        action: &str,
        key: &Key,
        environment: &Environment,
        policy: Reuse,
    ) -> Result<Option<Cached>, XtaskError> {
        if environment.reuse(policy) == Reuse::Never {
            return Ok(None);
        }
        let cached = match self {
            Self::Local(root) => read_local(&managed_path(root, &local_entry(action, key))?)?,
            Self::Remote(remote) => remote.restore(action, key, environment)?,
        };
        Ok(cached.filter(|cached| may_reuse(policy, cached, key)))
    }

    /// Publish a green verdict. There is deliberately no save operation for
    /// refused or broken outcomes.
    pub(crate) fn save_green(
        &self,
        action: &str,
        key: &Key,
        environment: &Environment,
    ) -> Result<(), XtaskError> {
        if environment.verifier.is_none() {
            return Err(XtaskError::Doc {
                file: "verification cache (verifier-unavailable)".to_owned(),
                msg:
                    "execute the check; this platform cannot identify the loaded verifier for reuse"
                        .to_owned(),
            });
        }
        let cached = Cached {
            key: key.as_str().to_owned(),
            verdict: Verdict::Green,
        };
        match self {
            Self::Local(root) => {
                write_local(&managed_path(root, &local_entry(action, key))?, &cached)
            }
            Self::Remote(remote) => remote.save(action, key, environment, &cached),
        }
    }
}

fn local_entry(action: &str, key: &Key) -> PathBuf {
    Path::new(action).join(format!("{}.json", key.as_str()))
}

fn managed_path(root: &Path, relative: &Path) -> Result<PathBuf, XtaskError> {
    let check = |path: &Path| match std::fs::symlink_metadata(path) {
        Ok(metadata) if crate::walk::is_alias(&metadata) => Err(store_in_source(path)),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(XtaskError::Io {
            path: path.display().to_string(),
            source,
        }),
    };
    let mut path = root.to_path_buf();
    check(&path)?;
    for part in relative.components() {
        let Component::Normal(name) = part else {
            return Err(store_in_source(&root.join(relative)));
        };
        path.push(name);
        check(&path)?;
    }
    Ok(path)
}

fn read_local(path: &Path) -> Result<Option<Cached>, XtaskError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(XtaskError::Io {
                path: path.display().to_string(),
                source,
            });
        }
    };
    match serde_json::from_slice(&bytes) {
        Ok(cached) => Ok(Some(cached)),
        Err(error) => {
            Diagnostic {
                code: "verdict-corrupt",
                action: "restore cached verification",
                subject: "stored verification verdict",
                path,
                value: &error.to_string(),
                rule: "a complete JSON verdict with an exact green content key",
                repair: "Run just verify-sources to compute a replacement for the corrupt entry.",
            }
            .emit();
            Ok(None)
        }
    }
}

/// Resolve the existing prefix before checking the scan boundary, so a
/// directory alias outside the checkout cannot write back into source.
pub(super) fn output_path(root: &Path, requested: &Path) -> Result<PathBuf, XtaskError> {
    let path = root.join(requested);
    if path
        .components()
        .any(|part| part == std::path::Component::ParentDir)
    {
        return Err(store_in_source(&path));
    }
    let mut prefix = path.as_path();
    let mut tail = Vec::new();
    loop {
        match std::fs::symlink_metadata(prefix) {
            Ok(_) => break,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(XtaskError::Io {
                    path: prefix.display().to_string(),
                    source,
                });
            }
        }
        let name = prefix.file_name().ok_or_else(|| store_in_source(&path))?;
        tail.push(name);
        prefix = prefix.parent().ok_or_else(|| store_in_source(&path))?;
    }
    let mut resolved = prefix.canonicalize().map_err(|source| XtaskError::Io {
        path: prefix.display().to_string(),
        source,
    })?;
    for name in tail.into_iter().rev() {
        resolved.push(name);
    }
    let root = root.canonicalize().map_err(|source| XtaskError::Io {
        path: root.display().to_string(),
        source,
    })?;
    if let Ok(relative) = resolved.strip_prefix(&root) {
        let relative = relative
            .to_str()
            .ok_or_else(|| store_in_source(&resolved))?;
        #[cfg(windows)]
        let relative = relative.replace('\\', "/");
        if !crate::walk::excluded_source_path(&format!("{relative}/verdict.json")) {
            return Err(store_in_source(&resolved));
        }
    }
    Ok(resolved)
}

fn store_in_source(path: &Path) -> XtaskError {
    XtaskError::Doc {
        file: format!("{} (store-in-source)", path.display()),
        msg: "keep target as an ordinary build directory or a link outside source; choose --store target/ci/verdicts or an absolute external directory without parent components".to_owned(),
    }
}

fn write_local(path: &Path, cached: &Cached) -> Result<(), XtaskError> {
    let parent = path.parent().ok_or_else(|| XtaskError::Doc {
        file: path.display().to_string(),
        msg: "the verdict path has no parent directory (verdict-root)".to_owned(),
    })?;
    std::fs::create_dir_all(parent).map_err(|source| XtaskError::Io {
        path: parent.display().to_string(),
        source,
    })?;
    if read_local(path)?.is_some() {
        return Ok(());
    }
    let bytes = serde_json::to_vec(cached).map_err(|err| XtaskError::Doc {
        file: path.display().to_string(),
        msg: format!("serialize a green verdict: {err} (verdict-serialize)"),
    })?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|source| XtaskError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    temporary
        .write_all(&bytes)
        .map_err(|source| XtaskError::Io {
            path: temporary.path().display().to_string(),
            source,
        })?;
    match temporary.persist(path) {
        Ok(_file) => Ok(()),
        Err(_already_exists) if path.is_file() => Ok(()),
        Err(error) => Err(XtaskError::Io {
            path: path.display().to_string(),
            source: error.error,
        }),
    }
}

fn remote_entry(action: &str, key: &Key) -> PathBuf {
    Path::new(action).join(key.as_str())
}

pub(crate) fn remote_key(action: &str, key: &Key, environment: &Environment) -> String {
    format!("ci-action-{action}-{}", key.cache_key(environment))
}

impl RemoteStore {
    #[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
    fn restore(
        &self,
        action: &str,
        key: &Key,
        environment: &Environment,
    ) -> Result<Option<Cached>, XtaskError> {
        let entry = remote_entry(action, key);
        let dir = managed_path(&self.work, &entry)?;
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|source| XtaskError::Io {
                path: dir.display().to_string(),
                source,
            })?;
        }
        std::fs::create_dir_all(&dir).map_err(|source| XtaskError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let output = Command::new("bun")
            .arg(&self.adapter)
            .arg("restore")
            .arg(remote_key(action, key, environment))
            .arg(&dir)
            .output()
            .map_err(|source| XtaskError::Io {
                path: self.adapter.display().to_string(),
                source,
            })?;
        if !output.status.success() {
            return Err(XtaskError::Cmd {
                cmd: "ci-cache restore".to_owned(),
                msg: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        if String::from_utf8_lossy(&output.stdout).lines().last() == Some("hit") {
            read_local(&managed_path(&self.work, &entry.join("verdict.json"))?)
        } else {
            Ok(None)
        }
    }

    #[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
    fn save(
        &self,
        action: &str,
        key: &Key,
        environment: &Environment,
        cached: &Cached,
    ) -> Result<(), XtaskError> {
        let entry = remote_entry(action, key);
        let dir = managed_path(&self.work, &entry)?;
        write_local(
            &managed_path(&self.work, &entry.join("verdict.json"))?,
            cached,
        )?;
        let output = Command::new("bun")
            .arg(&self.adapter)
            .arg("save")
            .arg(remote_key(action, key, environment))
            .arg(&dir)
            .output()
            .map_err(|source| XtaskError::Io {
                path: self.adapter.display().to_string(),
                source,
            })?;
        if output.status.success() {
            Ok(())
        } else {
            Err(XtaskError::Cmd {
                cmd: "ci-cache save".to_owned(),
                msg: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
