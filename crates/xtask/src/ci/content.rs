// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Canonical content identity for one verification node (xtask `spec/Ci.lean`).

use std::collections::BTreeMap;
#[cfg(any(test, target_os = "linux"))]
use std::path::Path;

use super::view::Snapshot;
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
                eprintln!(
                    "```toml\ncode = \"verifier-unavailable\"\naction = \"identify the loaded verifier for result reuse\"\nsubject = \"running image\"\npath = \"/proc/self/exe\"\nvalue = {:?}\nrule = \"only an identified loaded image may reuse verdicts\"\nrepair = \"continue with real verification; result reuse stays disabled\"\n```",
                    error.to_string()
                );
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
    for (name, value) in identity.variables {
        text(&mut hash, name)?;
        text(&mut hash, value)?;
    }
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
            verifier: Some("test-verifier".to_owned()),
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
            variables: empty_variables(),
        }
    }

    fn empty_variables() -> &'static BTreeMap<String, String> {
        static VARIABLES: BTreeMap<String, String> = BTreeMap::new();
        &VARIABLES
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
                    verifier: Some("test-verifier".to_owned()),
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

    #[test]
    fn a_captured_bad_header_cannot_borrow_a_later_good_header() {
        let root = fixture("header-view");
        let file = root.join("src/lib.rs");
        let bad = "fn captured() {}\n";
        std::fs::write(&file, bad).unwrap();
        let captured = Snapshot::capture(&root).unwrap();
        let good = crate::header::notice(crate::header::Leader::Rust)
            .into_iter()
            .chain(["".to_owned(), "fn captured() {}".to_owned()])
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&file, good).unwrap();
        let findings = crate::header::check(captured.root()).unwrap();
        std::fs::write(&file, bad).unwrap();
        captured.close().unwrap();
        std::fs::remove_dir_all(root).unwrap();
        assert_eq!(
            findings.len(),
            1,
            "the real header gate must read captured A, even after the original became B then A"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_replaced_executable_path_does_not_replace_the_loaded_identity() {
        const CHILD: &str = "XTASK_CI_IMAGE_REPLACEMENT_TEST";
        if std::env::var_os(CHILD).is_some() {
            let before = Environment::current();
            assert!(before.verifier.is_some());
            let executable = std::env::current_exe().unwrap();
            std::fs::rename(&executable, executable.with_extension("running")).unwrap();
            std::fs::write(&executable, b"replacement path bytes").unwrap();
            std::fs::remove_file(executable.with_extension("running")).unwrap();
            assert_eq!(Environment::current(), before);
            assert_ne!(
                before.verifier.unwrap(),
                blake3::hash(&std::fs::read(executable).unwrap())
                    .to_hex()
                    .to_string()
            );
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("verifier");
        std::fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
        let output = std::process::Command::new(executable)
            .args(["--exact", "ci::content::tests::a_replaced_executable_path_does_not_replace_the_loaded_identity", "--nocapture"])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        dir.close().unwrap();
    }

    #[test]
    fn unavailable_loaded_identity_disables_reuse() {
        let mut environment = environment();
        environment.verifier = None;
        assert_eq!(environment.reuse(Reuse::SuccessOnly), Reuse::Never);
        assert_eq!(environment.reuse(Reuse::Never), Reuse::Never);
    }

    #[test]
    fn loaded_verifier_and_explicit_process_inputs_change_the_key() {
        let root = fixture("explicit-identity");
        std::fs::write(root.join("src/lib.rs"), "fn same() {}\n").unwrap();
        let environment = environment();
        let dependencies = BTreeMap::new();
        let variables =
            BTreeMap::from([("SPRAWLING_CI_SOURCE_SHA".to_owned(), "first".to_owned())]);
        let first = key(
            &root,
            &action(),
            &Identity {
                base: &[],
                dependencies: &dependencies,
                environment: &environment,
                variables: &variables,
            },
        )
        .unwrap();
        let mut other = environment.clone();
        other.verifier = Some("other-loaded-verifier".to_owned());
        let changed_verifier = key(
            &root,
            &action(),
            &Identity {
                base: &[],
                dependencies: &dependencies,
                environment: &other,
                variables: &variables,
            },
        )
        .unwrap();
        let variables =
            BTreeMap::from([("SPRAWLING_CI_SOURCE_SHA".to_owned(), "second".to_owned())]);
        let changed_head = key(
            &root,
            &action(),
            &Identity {
                base: &[],
                dependencies: &dependencies,
                environment: &environment,
                variables: &variables,
            },
        )
        .unwrap();
        assert_ne!(first, changed_verifier);
        assert_ne!(first, changed_head);
        std::fs::remove_dir_all(root).unwrap();
    }
}
