// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Run every gate in a fixed order and aggregate the findings. Order is
//! cheap-and-local first, git last; violations from all gates are rendered
//! together so a builder sees the full list, not the first stumble.
//!
//! The array below is the only authority for which gates run and in what
//! order; `COUNT` is its length parameter, one token away. `report` owns
//! what a run's outcome reads like and what it exits with.

use std::path::Path;
use std::process::ExitCode;

use crate::report::{self, Violation, XtaskError};
use crate::{
    apisync, artifact, boundary, budget, color, depmap, docnum, guard, header, length, lexicon,
    modmap, npm, proof, release, render, secret, slices, specalign, wire_ts, wiring, wording,
};

/// How many gates run. The array below is typed by it, so the number and
/// the list are one token apart and cannot disagree; `vocabulary` reads
/// it so no document has to hold a copy.
pub(crate) const COUNT: usize = 23;

pub(crate) fn run(root: &Path, range: Option<&str>) -> ExitCode {
    let results: [(&'static str, Result<Vec<Violation>, XtaskError>); COUNT] = [
        ("header", header::check(root)),
        ("lexicon", lexicon::check(root)),
        ("modmap", modmap::check(root)),
        ("length", length::check(root)),
        ("boundary", boundary::check(root)),
        // `slices` judges which code may name one path, so it walks sources
        // the way `boundary` walks them and sits beside it.
        ("slices", slices::check(root)),
        ("artifact", artifact::check(root)),
        ("depmap", depmap::check(root)),
        ("npm", npm::check(root)),
        ("secret", secret::check(root)),
        ("color", color::check(root)),
        ("wording", wording::check(root)),
        ("render", render::check(root)),
        ("wiring", wiring::check(root)),
        // `wire-ts` renders the client's wire types in this process and
        // compares one file, so it is local and cheap; it sits beside
        // `wiring` because both judge the same socket seam.
        ("wire-ts", wire_ts::check(root)),
        // `docnum` judges the same documentation face `wire-ts` does,
        // and costs one scan of the markdown in the tree.
        ("docnum", docnum::check(root)),
        // `proof` reads source and two documents; it proves nothing
        // here, so it costs what a scan costs and belongs with them.
        ("proof", proof::check(root)),
        ("budget", budget::check(root)),
        ("specalign", specalign::check(root)),
        // `features` runs the compiler rather than reading source, so it
        // is the most expensive gate and sits with the other one that
        // spawns cargo. Its verdict is the default feature set's, test
        // targets included, which nothing else compiles.
        ("features", default_features(root)),
        ("apisync", apisync::check(root, range)),
        ("release", release::check(root)),
        ("guard", guard::check(root, range)),
    ];

    report::finish_all(results)
}

/// The default feature set, test targets included.
///
/// Every other build in this repository passes `--all-features`
/// (`clippy`, `nextest`) or builds one package (`dist`), so the
/// workspace on its own default features is a configuration nothing
/// else compiles. Code behind a feature is compiled the day somebody
/// turns that feature on, and a test target whose `cfg` does not match
/// the default set is exactly what this gate is for: one referenced
/// `#[cfg(feature = "conformance")]` items while declaring no such
/// gate, and no command but `--all-features` ever compiled it.
///
/// Judged by running the build rather than by reading source: the
/// verdict is the compiler's, and its diagnostics are on stderr by the
/// time this returns.
pub(crate) fn default_features(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let status = std::process::Command::new("cargo")
        .args(["check", "--workspace", "--locked", "--all-targets"])
        .current_dir(root)
        .status()
        .map_err(|source| XtaskError::Io {
            path: "run `cargo check --workspace --locked --all-targets`".to_owned(),
            source,
        })?;
    if status.success() {
        return Ok(Vec::new());
    }
    Ok(vec![Violation {
        gate: "features",
        location: "crates/**".to_owned(),
        rule: "the workspace builds on its default features, test targets included".to_owned(),
        violation: "`cargo check --workspace --locked --all-targets` failed".to_owned(),
        alternative: "fix the default-feature build; `--all-features` is a gate's configuration, \
                      not the one a person builds"
            .to_owned(),
    }])
}
