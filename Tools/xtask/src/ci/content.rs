// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Canonical content identity for one verification node (xtask `spec/Ci.lean`).

use std::collections::BTreeMap;
use std::path::Path;

use super::diagnostic::Diagnostic;
use super::view::{CapturedFile, Snapshot};
use crate::report::XtaskError;

use super::graph::{Action, Input, Operation};

const SCHEMA: &str = "xtask-ci-v1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Key(String);

impl Key {
    #[cfg(test)]
    pub(crate) fn named(name: String) -> Self {
        Self(name)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn cache_key(&self, environment: &Environment) -> String {
        format!(
            "ci-{SCHEMA}-{}-{}-{}",
            environment.os, environment.arch, self.0
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Environment {
    pub(crate) os: String,
    pub(crate) arch: String,
    pub(crate) verifier: Option<String>,
}

impl Environment {
    pub(crate) fn current() -> Self {
        Self::from_probe(running_image())
    }

    pub(super) fn from_probe(image: Result<Option<String>, XtaskError>) -> Self {
        let verifier = match image {
            Ok(identity) => identity,
            Err(error) => {
                Diagnostic {
                    code: "verifier-unavailable",
                    action: "identify the loaded verifier for result reuse",
                    subject: "running image",
                    path: Path::new("/proc/self/exe"),
                    value: &error.to_string(),
                    rule: "only an identified loaded image may reuse verdicts",
                    repair: "Run readlink /proc/self/exe. If procfs is unavailable, rerun just verify-sources inside a Linux environment with procfs mounted.",
                }
                .emit();
                None
            }
        };
        Self {
            os: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            verifier,
        }
    }

    pub(crate) fn reuse(&self, declared: super::graph::Reuse) -> super::graph::Reuse {
        if self.verifier.is_some() {
            declared
        } else {
            super::graph::Reuse::Never
        }
    }
}

/// Linux's procfs link opens the executing image, including after unlink or
/// rename. A pathname returned by current_exe does not offer that guarantee.
#[cfg(target_os = "linux")]
fn running_image() -> Result<Option<String>, XtaskError> {
    let path = Path::new("/proc/self/exe");
    let bytes = std::fs::read(path).map_err(|source| XtaskError::Io {
        path: "read the executing verifier through /proc/self/exe (verifier-identity)".to_owned(),
        source,
    })?;
    Ok(Some(blake3::hash(&bytes).to_hex().to_string()))
}

#[cfg(not(target_os = "linux"))]
fn running_image() -> Result<Option<String>, XtaskError> {
    Ok(None)
}

/// Inputs that always travel together when deriving a content key.
pub(crate) struct Identity<'a> {
    pub(crate) base: &'a [Input],
    pub(crate) dependencies: &'a BTreeMap<String, Key>,
    pub(crate) environment: &'a Environment,
    pub(crate) variables: &'a BTreeMap<String, String>,
    pub(crate) files: Option<&'a crate::source::Scope>,
}

/// Hash one node and all of its declared inputs; used by isolated fixture tests.
#[cfg(test)]
fn key(root: &Path, action: &Action, identity: &Identity) -> Result<Key, XtaskError> {
    let snapshot = Snapshot::capture(root)?;
    let result = key_in_snapshot(&snapshot, action, identity);
    snapshot.finish(result)
}

/// Derive a node key without rereading the checkout.
pub(crate) fn key_in_snapshot(
    snapshot: &Snapshot,
    action: &Action,
    identity: &Identity,
) -> Result<Key, XtaskError> {
    let specs: Vec<Input> = identity.base.iter().chain(action.inputs).copied().collect();
    let files = matching_files(snapshot, &specs)?;
    let mut hash = blake3::Hasher::new();
    text(&mut hash, SCHEMA)?;
    text(&mut hash, action.id)?;
    text(
        &mut hash,
        if action.reuse == super::graph::Reuse::SuccessOnly {
            "success-only"
        } else {
            "never"
        },
    )?;
    text(
        &mut hash,
        if action.required {
            "required"
        } else {
            "optional"
        },
    )?;
    text(&mut hash, &identity.environment.os)?;
    text(&mut hash, &identity.environment.arch)?;
    text(
        &mut hash,
        identity
            .environment
            .verifier
            .as_deref()
            .unwrap_or("unavailable"),
    )?;
    operation(&mut hash, action.operation)?;
    match identity.files {
        None => text(&mut hash, "full-inventory")?,
        Some(scope) => {
            text(&mut hash, "scoped-files")?;
            text(&mut hash, scope.paths().len().to_string())?;
            for path in scope.paths() {
                text(&mut hash, path)?;
            }
        }
    }
    for (name, value) in identity.variables {
        text(&mut hash, name)?;
        text(&mut hash, value)?;
    }
    for (id, key) in identity.dependencies {
        text(&mut hash, id)?;
        text(&mut hash, key.as_str())?;
    }
    for (path, file) in files {
        text(&mut hash, path)?;
        text(&mut hash, file.digest.as_bytes())?;
        text(
            &mut hash,
            match file.executable {
                Some(true) => b"x",
                Some(false) => b"-",
                None => b"?",
            },
        )?;
    }
    Ok(Key(hash.finalize().to_hex().to_string()))
}

fn matching_files<'a>(
    snapshot: &'a Snapshot,
    specs: &[Input],
) -> Result<BTreeMap<&'a String, &'a CapturedFile>, XtaskError> {
    let mut found = BTreeMap::new();
    for (path, digest) in snapshot.files() {
        if specs.iter().any(|spec| spec.matches(path)) {
            found.insert(path, digest);
        }
    }
    for spec in specs {
        if !found.keys().any(|path| spec.matches(path)) {
            return Err(XtaskError::Doc {
                file: format!("verification input {} (missing-input)", spec.root),
                msg: "declare only an input that exists, or make the producing dependency explicit"
                    .to_owned(),
            });
        }
    }
    Ok(found)
}

fn operation(hash: &mut blake3::Hasher, operation: Operation) -> Result<(), XtaskError> {
    match operation {
        Operation::Gate(name) => {
            text(hash, "gate")?;
            text(hash, name)
        }
        Operation::ScopedGate(name) => {
            text(hash, "scoped-gate")?;
            text(hash, name)
        }
        Operation::Just(recipe) => {
            text(hash, "just")?;
            text(hash, recipe)
        }
        Operation::Process { program, args } => {
            text(hash, "process")?;
            text(hash, program)?;
            for arg in args {
                text(hash, arg)?;
            }
            Ok(())
        }
    }
}

fn text<T: AsRef<[u8]>>(hash: &mut blake3::Hasher, value: T) -> Result<(), XtaskError> {
    let bytes = value.as_ref();
    let len = u64::try_from(bytes.len()).map_err(|_err| XtaskError::Doc {
        file: "verification content key (input-too-large)".to_owned(),
        msg: "keep one input below u64::MAX bytes".to_owned(),
    })?;
    hash.update(&len.to_le_bytes());
    hash.update(bytes);
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
