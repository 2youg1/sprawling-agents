// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Canonical content identity for one verification node (xtask `spec/Ci.lean`).

use std::collections::BTreeMap;
use std::path::Path;

use crate::report::XtaskError;
use crate::walk;

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
}

impl Environment {
    pub(crate) fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
        }
    }
}

/// One immutable view of the checkout; every file is hashed at most once per run.
#[derive(Debug)]
pub(crate) struct Snapshot {
    files: BTreeMap<String, blake3::Hash>,
}

impl Snapshot {
    pub(crate) fn capture(root: &Path) -> Result<Self, XtaskError> {
        let mut files = BTreeMap::new();
        for file in walk::files(root)? {
            let path = walk::rel(root, &file);
            if walk::in_isolation_zone(&path) {
                continue;
            }
            let bytes = std::fs::read(&file).map_err(|source| XtaskError::Io {
                path: path.clone(),
                source,
            })?;
            files.insert(path, blake3::hash(&bytes));
        }
        Ok(Self { files })
    }
}

/// Inputs that always travel together when deriving a content key.
pub(crate) struct Identity<'a> {
    pub(crate) base: &'a [Input],
    pub(crate) dependencies: &'a BTreeMap<String, Key>,
    pub(crate) environment: &'a Environment,
}

/// Hash one node and all of its declared inputs; used by isolated fixture tests.
#[cfg(test)]
fn key(root: &Path, action: &Action, identity: &Identity) -> Result<Key, XtaskError> {
    key_in_snapshot(&Snapshot::capture(root)?, action, identity)
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
    operation(&mut hash, action.operation)?;
    for (id, key) in identity.dependencies {
        text(&mut hash, id)?;
        text(&mut hash, key.as_str())?;
    }
    for (path, digest) in files {
        text(&mut hash, path)?;
        text(&mut hash, digest.as_bytes())?;
    }
    Ok(Key(hash.finalize().to_hex().to_string()))
}

fn matching_files<'a>(
    snapshot: &'a Snapshot,
    specs: &[Input],
) -> Result<BTreeMap<&'a String, &'a blake3::Hash>, XtaskError> {
    let mut found = BTreeMap::new();
    for (path, digest) in &snapshot.files {
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
mod tests {
    use std::path::PathBuf;

    use super::super::graph::Reuse;
    use super::*;

    const ACTION_INPUTS: &[Input] = &[Input::only("src", &["rs"])];

    fn fixture(label: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("xtask-ci-content-{label}-{}", std::process::id()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        root
    }

    fn action() -> Action {
        Action {
            id: "unit",
            operation: Operation::Gate("header"),
            inputs: ACTION_INPUTS,
            needs: &[],
            reuse: Reuse::SuccessOnly,
            required: true,
        }
    }

    fn environment() -> Environment {
        Environment {
            os: "test-os".to_owned(),
            arch: "test-arch".to_owned(),
        }
    }

    fn identity<'a>(
        dependencies: &'a BTreeMap<String, Key>,
        environment: &'a Environment,
    ) -> Identity<'a> {
        Identity {
            base: &[],
            dependencies,
            environment,
        }
    }

    #[test]
    fn equal_bytes_have_the_same_key_and_changed_bytes_move_it() {
        let root = fixture("bytes");
        std::fs::write(root.join("src/lib.rs"), "fn first() {}\n").unwrap();
        let before = key(
            &root,
            &action(),
            &identity(&BTreeMap::new(), &environment()),
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), "fn second() {}\n").unwrap();
        let after = key(
            &root,
            &action(),
            &identity(&BTreeMap::new(), &environment()),
        )
        .unwrap();
        std::fs::remove_dir_all(root).unwrap();
        assert_ne!(before, after);
    }

    #[test]
    fn paths_dependencies_and_environment_are_part_of_the_key() {
        let root = fixture("identity");
        std::fs::write(root.join("src/lib.rs"), "fn same() {}\n").unwrap();
        let base = key(
            &root,
            &action(),
            &identity(&BTreeMap::new(), &environment()),
        )
        .unwrap();
        let other_root = fixture("identity-other");
        std::fs::write(other_root.join("src/other.rs"), "fn same() {}\n").unwrap();
        let other_path = key(
            &other_root,
            &action(),
            &identity(&BTreeMap::new(), &environment()),
        )
        .unwrap();
        let with_dependency = key(
            &root,
            &action(),
            &identity(
                &BTreeMap::from([("base".to_owned(), Key::named("digest".to_owned()))]),
                &environment(),
            ),
        )
        .unwrap();
        let other_environment = key(
            &root,
            &action(),
            &identity(
                &BTreeMap::new(),
                &Environment {
                    os: "other".to_owned(),
                    arch: "test-arch".to_owned(),
                },
            ),
        )
        .unwrap();
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(other_root).unwrap();
        assert_ne!(base, other_path);
        assert_ne!(base, with_dependency);
        assert_ne!(base, other_environment);
    }

    #[test]
    fn a_missing_declared_input_is_refused() {
        let root = fixture("missing");
        let verdict = key(
            &root,
            &action(),
            &identity(&BTreeMap::new(), &environment()),
        );
        std::fs::remove_dir_all(root).unwrap();
        assert!(verdict.is_err());
    }
}
