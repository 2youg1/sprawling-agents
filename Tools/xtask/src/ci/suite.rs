// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The production source-verification graph. Inputs default to the whole
//! repository where a narrower ownership claim is not explicit (xtask D47).

use std::collections::BTreeMap;

use crate::report::XtaskError;

use super::content::{Environment, Identity, Key};
use super::graph::{Action, Input, Operation, Reuse};
use super::view::Snapshot;

/// Engine, command and toolchain inputs shared by every node.
pub(crate) const BASE_INPUTS: &[Input] = &[
    Input::all("Cargo.toml"),
    Input::all("Cargo.lock"),
    Input::all("rust-toolchain.toml"),
    Input::all("clippy.toml"),
    Input::all("deny.toml"),
    Input::all("justfile"),
    Input::all("bunfig.toml"),
    Input::all("Tools/xtask"),
    Input::all("Sprawling/desktop/ffi/zig-version"),
    Input::all("Calyx/ffi/zig-version"),
    Input::all(".github/ci-cache"),
];

const RUST: &[Input] = &[Input::only("", &["rs", "zig"])];
const CLIENT: &[Input] = &[Input::all("RefRain")];
const ALL: &[Input] = &[Input::all("")];
const LEXICON: &[Input] = &[
    Input::only("", &["md", "rs", "lean"]),
    Input::all(concat!(crate::walk::client_src!(), "/lang.json")),
];
const WIRE_TS: &[Input] = &[
    Input::all("Sprawling/wire"),
    Input::all("Sprawling/kernel"),
    Input::all("Sprawling/documents"),
    Input::all(crate::walk::CLIENT_SRC),
];
const CI_TOOLS_TEST: &[Input] = &[
    Input::all(".github/actions/ci-tools"),
    Input::all(".github/workflows/test-ci-tools.sh"),
    Input::all(".github/workflows/ci.yml"),
    Input::all(".github/actions/client-artifacts/action.yml"),
    Input::all(".github/workflows/cache-prune.yml"),
];
const NIGHTLY_TEST: &[Input] = &[
    Input::all(".github/workflows/test-nightly-release.sh"),
    Input::all(".github/workflows/nightly-release.yml"),
    Input::all(".github/workflows/release.yml"),
    Input::all(".github/workflows/nightly.yml"),
];
const CHANNEL_TEST: &[Input] = &[Input::all("Tools/xtask/src/channel")];
const DESKTOP_ZIG: &[Input] = &[Input::all("Sprawling/desktop/ffi")];
const CONSOLE_ZIG: &[Input] = &[Input::all("Calyx/ffi")];

const fn gate(id: &'static str) -> Action {
    Action {
        id,
        operation: Operation::Gate(id),
        inputs: ALL,
        needs: &[],
        reuse: Reuse::Never,
        required: true,
    }
}

/// Only a gate whose complete dependency surface is the owned source view
/// and linked verifier may opt into reuse. New gates remain non-reusable.
const fn pure(id: &'static str, inputs: &'static [Input]) -> Action {
    Action {
        inputs,
        reuse: Reuse::SuccessOnly,
        ..gate(id)
    }
}

const fn scoped(id: &'static str) -> Action {
    Action {
        operation: Operation::ScopedGate(id),
        ..gate(id)
    }
}

const fn just(id: &'static str, recipe: &'static str, inputs: &'static [Input]) -> Action {
    Action {
        id,
        operation: Operation::Just(recipe),
        inputs,
        needs: &[],
        reuse: Reuse::Never,
        required: true,
    }
}

const fn process(
    id: &'static str,
    program: &'static str,
    args: &'static [&'static str],
    inputs: &'static [Input],
) -> Action {
    Action {
        id,
        operation: Operation::Process { program, args },
        inputs,
        needs: &[],
        reuse: Reuse::Never,
        required: true,
    }
}

/// Every verdict formerly folded into `gates-sources`, with its input ownership.
pub(crate) const ACTIONS: &[Action] = &[
    just("fmt", "fmt-check", ALL),
    pure("header", RUST),
    pure("lexicon", LEXICON),
    gate("modmap"),
    scoped("length"),
    scoped("comments"),
    gate("boundary"),
    gate("artifact"),
    gate("residue"),
    // Any file can be named by an include directive; narrowing this input to
    // Rust sources would miss a deleted included document.
    gate("packaged"),
    gate("depmap"),
    gate("unused"),
    gate("secret"),
    gate("color"),
    pure("motion", CLIENT),
    pure("wording", CLIENT),
    gate("wiring"),
    pure("wire-ts", WIRE_TS),
    gate("docnum"),
    gate("proof"),
    gate("specalign"),
    gate("spec"),
    pure("release", ALL),
    gate("guard"),
    process(
        "channel-validation",
        "python",
        &["Tools/xtask/src/channel/validate_test.py"],
        CHANNEL_TEST,
    ),
    process(
        "ci-tools-contract",
        "bash",
        &[".github/workflows/test-ci-tools.sh"],
        CI_TOOLS_TEST,
    ),
    process(
        "nightly-contract",
        "sh",
        &[".github/workflows/test-nightly-release.sh"],
        NIGHTLY_TEST,
    ),
    just("desktop-zig", "check-desktop", DESKTOP_ZIG),
    just("console-zig", "check-console", CONSOLE_ZIG),
];

#[derive(Debug)]
pub(crate) struct PreparedAction {
    pub(crate) action: &'static Action,
    pub(crate) key: Key,
    pub(crate) variables: BTreeMap<String, String>,
    pub(crate) files: Option<crate::source::Scope>,
}

/// Capture once, validate the graph, and derive every key in prerequisite order.
pub(crate) fn prepare(
    snapshot: &Snapshot,
    environment: &Environment,
    source_sha: &str,
    files: Option<&crate::source::Scope>,
) -> Result<Vec<PreparedAction>, XtaskError> {
    super::graph::validate(ACTIONS)?;
    let mut keys: BTreeMap<String, Key> = BTreeMap::new();
    while keys.len() < ACTIONS.len() {
        let before = keys.len();
        for action in ACTIONS {
            if keys.contains_key(action.id) {
                continue;
            }
            let dependencies: Option<BTreeMap<String, Key>> = action
                .needs
                .iter()
                .map(|need| keys.get(*need).map(|key| ((*need).to_owned(), key.clone())))
                .collect();
            if let Some(dependencies) = dependencies {
                let variables = variables(action, source_sha);
                let key = super::content::key_in_snapshot(
                    snapshot,
                    action,
                    &Identity {
                        base: BASE_INPUTS,
                        dependencies: &dependencies,
                        environment,
                        variables: &variables,
                        files: file_scope(action, files),
                    },
                )?;
                keys.insert(action.id.to_owned(), key);
            }
        }
        if keys.len() == before {
            return Err(XtaskError::Doc {
                file: "verification graph (missing-prerequisite-key)".to_owned(),
                msg: "the validated graph made no progress; report this as a CI engine defect"
                    .to_owned(),
            });
        }
    }
    ACTIONS
        .iter()
        .map(|action| {
            let key = keys
                .get(action.id)
                .cloned()
                .ok_or_else(|| XtaskError::Doc {
                    file: format!("verification node {} (missing-content-key)", action.id),
                    msg: "derive the node's key before executing it".to_owned(),
                })?;
            Ok(PreparedAction {
                action,
                key,
                variables: variables(action, source_sha),
                files: file_scope(action, files).cloned(),
            })
        })
        .collect()
}

fn file_scope<'a>(
    action: &Action,
    files: Option<&'a crate::source::Scope>,
) -> Option<&'a crate::source::Scope> {
    if matches!(action.operation, Operation::ScopedGate(_)) {
        files
    } else {
        None
    }
}

fn variables(action: &Action, source_sha: &str) -> BTreeMap<String, String> {
    if action.id == "nightly-contract" {
        BTreeMap::from([("SPRAWLING_CI_SOURCE_SHA".to_owned(), source_sha.to_owned())])
    } else {
        BTreeMap::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn the_source_slice_contains_every_source_gate_exactly_once() {
        let expected: BTreeSet<&str> = crate::gates::GATES
            .iter()
            .map(|gate| gate.name)
            .filter(|name| !["render", "npm", "budget"].contains(name))
            .collect();
        let actual: BTreeSet<&str> = ACTIONS
            .iter()
            .filter_map(|action| match action.operation {
                Operation::Gate(name) | Operation::ScopedGate(name) => Some(name),
                _ => None,
            })
            .collect();
        assert_eq!(actual, expected);
        assert_eq!(
            actual.len(),
            ACTIONS
                .iter()
                .filter(|action| matches!(
                    action.operation,
                    Operation::Gate(_) | Operation::ScopedGate(_)
                ))
                .count()
        );
    }

    #[test]
    fn the_production_graph_is_acyclic_and_complete() {
        assert!(crate::ci::graph::validate(ACTIONS).is_ok());
        assert!(ACTIONS.iter().all(|action| action.required));
    }

    #[test]
    fn only_nightly_contract_receives_the_captured_head() {
        for action in ACTIONS {
            let inputs = variables(action, "fixed-head");
            if action.id == "nightly-contract" {
                assert_eq!(
                    inputs.get("SPRAWLING_CI_SOURCE_SHA").map(String::as_str),
                    Some("fixed-head")
                );
            } else {
                assert!(inputs.is_empty());
            }
        }
    }

    #[test]
    fn cargo_backed_gates_select_top_level_members_and_both_config_names() {
        for path in [
            "Mei/Cargo.toml",
            "future-member/Cargo.toml",
            ".cargo/config.toml",
            ".cargo/config",
        ] {
            let affected = super::super::graph::affected_with_base(
                ACTIONS,
                BASE_INPUTS,
                &BTreeSet::from([path.to_owned()]),
            );
            for action in ACTIONS.iter().filter(|action| {
                matches!(action.operation, Operation::Gate(_)) && action.reuse == Reuse::Never
            }) {
                assert!(
                    affected.contains(action.id),
                    "{path} must select {}",
                    action.id
                );
            }
        }
    }

    #[test]
    fn formerly_missing_inputs_change_production_keys_and_selection() {
        let cases = [
            ("fmt", ".github/workflows/format-input.yml"),
            ("fmt", ".gitignore"),
            ("ci-tools-contract", ".github/workflows/ci.yml"),
            (
                "ci-tools-contract",
                ".github/actions/client-artifacts/action.yml",
            ),
            ("ci-tools-contract", ".github/workflows/cache-prune.yml"),
            ("nightly-contract", ".github/workflows/nightly.yml"),
        ];
        for (id, changed) in cases {
            let action = ACTIONS.iter().find(|action| action.id == id).unwrap();
            let root = tempfile::tempdir().unwrap();
            for input in BASE_INPUTS.iter().chain(action.inputs) {
                if input.root.is_empty() {
                    continue;
                }
                let mut file = root.path().join(input.root);
                if [
                    "Tools/xtask",
                    ".github/ci-cache",
                    ".github/actions/ci-tools",
                ]
                .contains(&input.root)
                {
                    file = file.join("fixture.rs");
                }
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(file, "fixture\n").unwrap();
            }
            let file = root.path().join(changed);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, "before\n").unwrap();
            let before = Snapshot::capture(root.path()).unwrap();
            std::fs::write(file, "after\n").unwrap();
            let after = Snapshot::capture(root.path()).unwrap();
            let dependencies = BTreeMap::new();
            let environment = Environment {
                os: "fixture".to_owned(),
                arch: "fixture".to_owned(),
                verifier: Some("fixture-verifier".to_owned()),
            };
            let variables = variables(action, "fixed-head");
            let identity = Identity {
                base: BASE_INPUTS,
                dependencies: &dependencies,
                environment: &environment,
                variables: &variables,
                files: None,
            };
            assert_ne!(
                super::super::content::key_in_snapshot(&before, action, &identity).unwrap(),
                super::super::content::key_in_snapshot(&after, action, &identity).unwrap(),
                "{changed} must change {id}'s production key"
            );
            assert!(
                super::super::graph::affected_with_base(
                    ACTIONS,
                    BASE_INPUTS,
                    &BTreeSet::from([changed.to_owned()])
                )
                .contains(id)
            );
            before.close().unwrap();
            after.close().unwrap();
            root.close().unwrap();
        }
    }
}
