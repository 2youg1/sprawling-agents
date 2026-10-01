// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `specalign` once the kernel's specification is Lean (tools/xtask/Spec.lean
//! §8-43): the carrier of every `AxCode`, the window class of
//! every `EventKind`, and every `inductive` that shares a name with a
//! kernel enum, each against what the kernel compiles.
//!
//! The Markdown SPEC wrote the first two as tables and the rosters as
//! Rust fences; the Lean specification writes them as a `def` with one
//! arm per line and as an `inductive` with one constructor per line. The
//! names are the Rust names, spelled the same, so the comparison is one
//! of equal strings. An `inductive` is never abbreviated, so no roster
//! is skipped as a pointer elsewhere.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kernel::{AxCode, Carrier, EventKind, WindowClass};

use crate::lean::{self, Source, Table};
use crate::report::{Violation, XtaskError};

/// The two tables, by the `def` that holds each and the enum it covers.
const CARRIERS: &str = "AxCode.carrier";
const WINDOWS: &str = "EventKind.windowClass";

pub(super) fn check(
    root: &Path,
    sources: &[Source],
    out: &mut Vec<Violation>,
) -> Result<(), XtaskError> {
    let carriers: Vec<(String, String)> = AxCode::ALL
        .iter()
        .map(|code| (format!("{code:?}"), carrier_of(*code)))
        .collect();
    compare(&located(sources, CARRIERS)?, &carriers, CARRIERS, out);
    let windows: Vec<(String, String)> = EventKind::ALL
        .iter()
        .map(|kind| (format!("{kind:?}"), window_of(*kind)))
        .collect();
    compare(&located(sources, WINDOWS)?, &windows, WINDOWS, out);
    rosters(root, sources, out)
}

/// The right-hand side the carrier table owes `code`.
fn carrier_of(code: AxCode) -> String {
    match code.carrier() {
        Carrier::Loadtime => ".loadtime".to_owned(),
        Carrier::Event(kind) => format!(".event .{kind:?}"),
    }
}

/// The right-hand side the window table owes `kind`.
fn window_of(kind: EventKind) -> String {
    match kind.window_class() {
        WindowClass::InWindow => ".inWindow".to_owned(),
        WindowClass::RecordOnly => ".recordOnly".to_owned(),
    }
}

/// A table, and the file it was read from.
struct Located {
    path: String,
    table: Table,
}

/// The table `def` names, from whichever file of the specification
/// holds it.
///
/// # Errors
/// When no file does: a gate that found no table would otherwise pass
/// every variant.
fn located(sources: &[Source], def: &str) -> Result<Located, XtaskError> {
    sources
        .iter()
        .find_map(|source| {
            lean::arms(&lean::code(&source.text), def).map(|table| Located {
                path: source.path.clone(),
                table,
            })
        })
        .ok_or_else(|| XtaskError::Doc {
            file: "the kernel's Lean specification".to_owned(),
            msg: format!(
                "no `def {def}` written one arm per line (tools/xtask/Spec.lean §8-43); write it \
                 there, so the gate has a table to hold the enum to"
            ),
        })
}

/// Every variant has exactly the arm the kernel owes it, and every arm
/// is a variant.
fn compare(found: &Located, expected: &[(String, String)], def: &str, out: &mut Vec<Violation>) {
    let mut arms: BTreeMap<&str, (usize, &str)> = found
        .table
        .arms
        .iter()
        .map(|arm| (arm.pattern.as_str(), (arm.line, arm.value.as_str())))
        .collect();
    for (variant, owed) in expected {
        match arms.remove(variant.as_str()) {
            None => out.push(finding(
                format!("{}:{}", found.path, found.table.line),
                format!("`{variant}` is in the kernel and has no arm in `{def}`"),
                "add the arm in the change-set that adds the variant",
            )),
            Some((line, stated)) if stated != owed => out.push(finding(
                format!("{}:{line}", found.path),
                format!("`{variant}`: `{def}` says `{stated}`, the kernel says `{owed}`"),
                "fix whichever side no longer matches the other",
            )),
            Some(_) => {}
        }
    }
    for (orphan, (line, _)) in arms {
        out.push(finding(
            format!("{}:{line}", found.path),
            format!("`{orphan}` has an arm in `{def}` and is not a variant in the kernel"),
            "remove the arm, or add the variant in the same change-set",
        ));
    }
}

/// Every `inductive` named like a kernel enum lists exactly its variants.
fn rosters(root: &Path, sources: &[Source], out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let compiled = super::enums::compiled(root)?;
    for source in sources {
        for (name, roster) in lean::inductives(&lean::code(&source.text)) {
            let Some(actual) = compiled.get(&name) else {
                continue;
            };
            let declared: BTreeSet<String> = roster.names.into_iter().collect();
            let at = format!("{}:{}", source.path, roster.line);
            for absent in declared.difference(actual) {
                out.push(finding(
                    at.clone(),
                    format!("`{name}.{absent}` is in the specification and not in the kernel"),
                    "delete the constructor, or add the variant to the enum",
                ));
            }
            for extra in actual.difference(&declared) {
                out.push(finding(
                    at.clone(),
                    format!("`{name}::{extra}` is in the kernel and not in the specification"),
                    "add the constructor in the change-set that adds the variant",
                ));
            }
        }
    }
    Ok(())
}

fn finding(location: String, violation: String, alternative: &str) -> Violation {
    Violation {
        gate: "specalign",
        location,
        rule: "the kernel's Lean specification names exactly the variants the kernel compiles, \
               with the carrier and window class each one has (tools/xtask/Spec.lean §8-43)"
            .to_owned(),
        violation,
        alternative: alternative.to_owned(),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]
mod tests {
    use crate::root::fixture;

    /// Every carrier arm the real kernel owes, with one left out and one
    /// changed, and a roster one constructor short.
    #[test]
    fn a_lean_kernel_specification_is_held_to_the_compiled_enums() {
        let root = fixture::relocated("specalign-tables");
        let mut carriers: Vec<String> = kernel::AxCode::ALL
            .iter()
            .map(|code| format!("  | .{code:?} => {}", super::carrier_of(*code)))
            .collect();
        carriers.retain(|arm| !arm.contains(".InvalidArgs "));
        let windows: Vec<String> = kernel::EventKind::ALL
            .iter()
            .map(|kind| match kind {
                kernel::EventKind::ToolResult => "  | .ToolResult => .recordOnly".to_owned(),
                other => format!("  | .{other:?} => {}", super::window_of(*other)),
            })
            .collect();
        let spec = format!(
            "-- a comment\ninductive Colour where\n  | Red\n  deriving Repr\n\n\
             def AxCode.carrier : AxCode → Carrier\n{}\n\n\
             def EventKind.windowClass : EventKind → WindowClass\n{}\n",
            carriers.join("\n"),
            windows.join("\n")
        );
        fixture::write(&root, "crates/kernel/Spec.lean", &spec);
        fixture::write(
            &root,
            "crates/kernel/src/lib.rs",
            "pub enum Colour { Red, Blue }\n",
        );
        let found = super::super::check(&root);
        std::fs::remove_dir_all(&root).unwrap();
        let texts = found
            .map(|all| {
                all.into_iter()
                    .filter(|v| v.location.starts_with("crates/kernel"))
                    .map(|v| v.violation)
                    .collect::<Vec<_>>()
            })
            .map_err(|err| err.to_string());
        assert_eq!(
            texts,
            Ok(vec![
                "`InvalidArgs` is in the kernel and has no arm in `AxCode.carrier`".to_owned(),
                "`ToolResult`: `EventKind.windowClass` says `.recordOnly`, the kernel says \
                 `.inWindow`"
                    .to_owned(),
                "`Colour::Blue` is in the kernel and not in the specification".to_owned(),
            ])
        );
    }
}
