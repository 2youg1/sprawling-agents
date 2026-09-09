// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The npm gate: the dependency face of `client/` (xtask-SPEC.md
//! section 8-12).
//!
//! `client/` arrived in card 6.1 without the gate that guards it. The
//! workspace side of the tree has `cargo-deny` and `depmap` watching
//! what it may depend on; the JavaScript side had nothing, so one
//! `bun add` could bring in a third runtime dependency, a package under
//! a licence this repository refuses, or a lockfile that no longer
//! agrees with the manifest — and a green `just check` would say none of
//! it.
//!
//! **Three assertions, each against a drift that costs something
//! different.** A lockfile out of step with the manifest means this
//! machine and CI install two different trees. A third runtime
//! dependency is weight in a person's browser that nobody decided to
//! add. A licence outside the allowlist is a legal fact discovered at
//! release time rather than at commit time.
//!
//! **The allowlist is not this gate's.** `deny.toml` already states what
//! this repository permits, for the workspace side; a repository has one
//! position on licences, so this gate reads that same table rather than
//! keeping a second copy that could drift from it.
//!
//! **A missing `node_modules` skips the third assertion and says so.**
//! It is a gitignored directory, so a machine that has not run
//! `bun install` does not have one, and that is not a defect. The first
//! two read committed files only and therefore judge everywhere: a gate
//! that falls silent as a whole when the environment is bare is a gate
//! that has stopped existing outside CI.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::report::{Violation, XtaskError};
use crate::walk;

mod lockfile;

use lockfile::Manifest;

/// The client this gate guards. Its absence is silence, not a finding.
const CLIENT: &str = "client";

/// What the client asks for.
const MANIFEST: &str = "client/package.json";

/// What bun solved from it.
const LOCKFILE: &str = "client/bun.lock";

/// The installed tree, which is where a licence is actually written.
const MODULES: &str = "client/node_modules";

/// The one place this repository states which licences it permits.
const PERMITTED: &str = "deny.toml";

/// What may reach a person's browser. Exactly these, in both
/// directions: a gate that only refused additions would wave through the
/// day `solid-js` is deleted by accident.
const RUNTIME: [&str; 2] = ["effect", "solid-js"];

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    if !root.join(CLIENT).is_dir() {
        return Ok(Vec::new());
    }
    let manifest_path = root.join(MANIFEST);
    if !manifest_path.is_file() {
        return Ok(vec![missing(MANIFEST, "the client has no manifest")]);
    }
    let lock_path = root.join(LOCKFILE);
    if !lock_path.is_file() {
        return Ok(vec![missing(
            LOCKFILE,
            "the lockfile is not in the tree, so nothing pins what gets installed",
        )]);
    }
    let manifest = lockfile::manifest_of(&lockfile::document(
        MANIFEST,
        &walk::read_text(&manifest_path)?,
    )?);
    let locked = lockfile::lock_of(&lockfile::document(
        LOCKFILE,
        &walk::read_text(&lock_path)?,
    )?)?;
    let mut violations = Vec::new();
    judge_lockfile(&manifest, &locked, &mut violations);
    judge_runtime(&manifest, &mut violations);
    let modules = root.join(MODULES);
    if modules.is_dir() {
        let permitted = lockfile::permitted(&walk::read_text(&root.join(PERMITTED))?)?;
        judge_licences(root, &modules, &permitted, &mut violations)?;
    } else {
        println!(
            "gate npm: {MODULES} is not installed; run `bun install` in client/ to judge licences (skipped)"
        );
    }
    Ok(violations)
}

/// A file this gate cannot do without.
fn missing(location: &str, what: &str) -> Violation {
    Violation {
        gate: "npm",
        location: location.to_owned(),
        rule: "the client's lockfile and manifest are both in the tree".to_owned(),
        violation: what.to_owned(),
        alternative: "run `bun install` in client/ and commit what it writes".to_owned(),
    }
}

/// The lockfile records what the manifest asks for, entry for entry.
fn judge_lockfile(manifest: &Manifest, locked: &Manifest, out: &mut Vec<Violation>) {
    compare("dependencies", &manifest.runtime, &locked.runtime, out);
    compare(
        "devDependencies",
        &manifest.development,
        &locked.development,
        out,
    );
}

/// One table of the manifest against the same table of the lockfile.
fn compare(
    table: &str,
    asked: &BTreeMap<String, String>,
    locked: &BTreeMap<String, String>,
    out: &mut Vec<Violation>,
) {
    for (name, range) in asked {
        match locked.get(name) {
            Some(held) if held == range => {}
            Some(held) => out.push(stale(format!(
                "{table}.{name} is {range:?} in the manifest and {held:?} in the lockfile"
            ))),
            None => out.push(stale(format!(
                "{table}.{name} is in the manifest and not in the lockfile"
            ))),
        }
    }
    for name in locked.keys() {
        if !asked.contains_key(name) {
            out.push(stale(format!(
                "{table}.{name} is in the lockfile and not in the manifest"
            )));
        }
    }
}

/// One wording for every way the two files disagree.
fn stale(what: String) -> Violation {
    Violation {
        gate: "npm",
        location: LOCKFILE.to_owned(),
        rule: "the lockfile states what the manifest asks for, entry for entry".to_owned(),
        violation: what,
        alternative: "run `bun install` in client/ and commit the lockfile it writes".to_owned(),
    }
}

/// The runtime dependencies are exactly the two that were decided on.
fn judge_runtime(manifest: &Manifest, out: &mut Vec<Violation>) {
    let allowed: BTreeSet<&str> = RUNTIME.into_iter().collect();
    let asked: BTreeSet<&str> = manifest.runtime.keys().map(String::as_str).collect();
    for extra in asked.difference(&allowed) {
        out.push(runtime_violation(format!(
            "{extra} is a runtime dependency and is not one of {}",
            RUNTIME.join(", ")
        )));
    }
    for absent in allowed.difference(&asked) {
        out.push(runtime_violation(format!(
            "{absent} is not a runtime dependency, and every screen this client draws needs it"
        )));
    }
}

/// One wording for both directions of the runtime allowlist.
fn runtime_violation(what: String) -> Violation {
    Violation {
        gate: "npm",
        location: MANIFEST.to_owned(),
        rule: format!(
            "the client's runtime dependencies are exactly {}",
            RUNTIME.join(" and ")
        ),
        violation: what,
        alternative: "put the package in devDependencies if it is toolchain, or record the \
                      decision in client-SPEC.md before it reaches a person's browser"
            .to_owned(),
    }
}

/// Every installed package states a licence this repository permits.
fn judge_licences(
    root: &Path,
    modules: &Path,
    permitted: &BTreeSet<String>,
    out: &mut Vec<Violation>,
) -> Result<(), XtaskError> {
    for (name, manifest) in installed(modules)? {
        let text = walk::read_text(&manifest)?;
        let Ok(document) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let stated = document
            .get("license")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if lockfile::allowed(stated, permitted) {
            continue;
        }
        let said = if stated.is_empty() {
            "states no `license` field this gate can read".to_owned()
        } else {
            format!("is licensed {stated:?}")
        };
        out.push(Violation {
            gate: "npm",
            location: walk::rel(root, &manifest),
            rule: format!(
                "every package in the client's tree is licensed under one of the \
                           licences {PERMITTED} permits"
            ),
            violation: format!("{name} {said}"),
            alternative: "replace the package, or add its licence to `[licenses] allow` in \
                          deny.toml with the reason - that table is the one position this \
                          repository has on licences"
                .to_owned(),
        });
    }
    Ok(())
}

/// Every installed package and the manifest that states its licence,
/// sorted, so two runs report the same findings in the same order.
///
/// Scopes hold packages rather than being packages, and a package may
/// carry a nested tree of its own when a version could not be hoisted;
/// both are walked, so a licence cannot hide one level down.
fn installed(modules: &Path) -> Result<Vec<(String, PathBuf)>, XtaskError> {
    let mut out = Vec::new();
    collect(modules, "", &mut out)?;
    out.sort();
    Ok(out)
}

fn collect(
    modules: &Path,
    scope: &str,
    out: &mut Vec<(String, PathBuf)>,
) -> Result<(), XtaskError> {
    let entries = std::fs::read_dir(modules).map_err(|source| XtaskError::Io {
        path: modules.to_string_lossy().into_owned(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| XtaskError::Io {
            path: modules.to_string_lossy().into_owned(),
            source,
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if scope.is_empty() && name.starts_with('@') {
            collect(&path, &name, out)?;
            continue;
        }
        let full = if scope.is_empty() {
            name
        } else {
            format!("{scope}/{name}")
        };
        let manifest = path.join("package.json");
        if manifest.is_file() {
            out.push((full, manifest));
        }
        let nested = path.join("node_modules");
        if nested.is_dir() {
            collect(&nested, "", out)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
