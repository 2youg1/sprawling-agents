// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The production source-verification graph. Inputs default to the whole
//! repository where a narrower ownership claim is not explicit (xtask D43).

use std::collections::BTreeMap;
use std::path::Path;

use crate::report::XtaskError;

use super::content::{Environment, Identity, Key, Snapshot};
use super::graph::{Action, Input, Operation, Reuse};

/// Engine, command and toolchain inputs shared by every node.
pub(crate) const BASE_INPUTS: &[Input] = &[
    Input::all("Cargo.toml"),
    Input::all("Cargo.lock"),
    Input::all("rust-toolchain.toml"),
    Input::all("clippy.toml"),
    Input::all("deny.toml"),
    Input::all("justfile"),
    Input::all("bunfig.toml"),
    Input::all("crates/xtask"),
    Input::all("crates/desktop/ffi/zig-version"),
    Input::all("crates/console_ffi/zig-version"),
    Input::all(".github/ci-cache"),
];

const RUST: &[Input] = &[Input::only("", &["rs", "zig"])];
const RUST_MANIFESTS: &[Input] = &[Input::only("", &["rs", "zig", "toml"])];
const LEAN_RUST_DOCS: &[Input] = &[Input::only("", &["rs", "zig", "lean", "md", "toml"])];
const CLIENT: &[Input] = &[Input::all("client")];
const ALL: &[Input] = &[Input::all("")];
const FORMATTABLE: &[Input] = &[Input::only(
    "",
    &["rs", "zig", "ts", "svelte", "js", "json", "css", "html"],
)];
const LEXICON: &[Input] = &[
    Input::only("", &["md", "rs", "lean"]),
    Input::all("client/src/lang.json"),
];
const WIRING: &[Input] = &[Input::all("client"), Input::all("crates")];
const WIRE_TS: &[Input] = &[
    Input::all("crates/wire"),
    Input::all("crates/kernel"),
    Input::all("crates/documents"),
    Input::all("client/src/wire.ts"),
];
const CI_TOOLS_TEST: &[Input] = &[
    Input::all(".github/actions/ci-tools"),
    Input::all(".github/workflows/test-ci-tools.sh"),
];
const NIGHTLY_TEST: &[Input] = &[
    Input::all(".github/workflows/test-nightly-release.sh"),
    Input::all(".github/workflows/nightly-release.yml"),
    Input::all(".github/workflows/release.yml"),
];
const CHANNEL_TEST: &[Input] = &[Input::all("crates/xtask/src/channel")];
const DESKTOP_ZIG: &[Input] = &[Input::all("crates/desktop/ffi")];
const CONSOLE_ZIG: &[Input] = &[Input::all("crates/console_ffi")];

const fn gate(id: &'static str, inputs: &'static [Input]) -> Action {
    Action {
        id,
        operation: Operation::Gate(id),
        inputs,
        needs: &[],
        reuse: Reuse::SuccessOnly,
        required: true,
    }
}

const fn just(id: &'static str, recipe: &'static str, inputs: &'static [Input]) -> Action {
    Action {
        id,
        operation: Operation::Just(recipe),
        inputs,
        needs: &[],
        reuse: Reuse::SuccessOnly,
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
        reuse: Reuse::SuccessOnly,
        required: true,
    }
}

/// Every verdict formerly folded into `gates-sources`, with its input ownership.
pub(crate) const ACTIONS: &[Action] = &[
    just("fmt", "fmt-check", FORMATTABLE),
    gate("header", RUST),
    gate("lexicon", LEXICON),
    gate("modmap", RUST_MANIFESTS),
    gate("length", RUST_MANIFESTS),
    gate("boundary", LEAN_RUST_DOCS),
    gate("artifact", ALL),
    // Any file can be named by an include directive; narrowing this input to
    // Rust sources would miss a deleted included document.
    gate("packaged", ALL),
    gate("depmap", LEAN_RUST_DOCS),
    gate("unused", RUST_MANIFESTS),
    gate("secret", ALL),
    gate("color", ALL),
    gate("motion", CLIENT),
    gate("wording", CLIENT),
    gate("wiring", WIRING),
    gate("wire-ts", WIRE_TS),
    gate("docnum", ALL),
    gate("proof", ALL),
    gate("specalign", LEAN_RUST_DOCS),
    gate("spec", ALL),
    gate("release", ALL),
    gate("guard", RUST_MANIFESTS),
    process(
        "channel-validation",
        "python",
        &["crates/xtask/src/channel/validate_test.py"],
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
    Action {
        id: "console-zig",
        operation: Operation::Just("check-console"),
        inputs: CONSOLE_ZIG,
        // Both recipes use target/zig-test; sharing it concurrently is not a
        // hermetic input, so this edge keeps the two writers ordered.
        needs: &["desktop-zig"],
        reuse: Reuse::SuccessOnly,
        required: true,
    },
];

#[derive(Debug)]
pub(crate) struct PreparedAction {
    pub(crate) action: &'static Action,
    pub(crate) key: Key,
}

/// Capture once, validate the graph, and derive every key in prerequisite order.
pub(crate) fn prepare(
    root: &Path,
    environment: &Environment,
) -> Result<Vec<PreparedAction>, XtaskError> {
    super::graph::validate(ACTIONS)?;
    let snapshot = Snapshot::capture(root)?;
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
                let key = super::content::key_in_snapshot(
                    &snapshot,
                    action,
                    &Identity {
                        base: BASE_INPUTS,
                        dependencies: &dependencies,
                        environment,
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
            Ok(PreparedAction { action, key })
        })
        .collect()
}

#[cfg(test)]
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
                Operation::Gate(name) => Some(name),
                _ => None,
            })
            .collect();
        assert_eq!(actual, expected);
        assert_eq!(
            actual.len(),
            ACTIONS
                .iter()
                .filter(|action| matches!(action.operation, Operation::Gate(_)))
                .count()
        );
    }

    #[test]
    fn the_production_graph_is_acyclic_and_complete() {
        assert!(crate::ci::graph::validate(ACTIONS).is_ok());
        assert!(ACTIONS.iter().all(|action| action.required));
    }
}
