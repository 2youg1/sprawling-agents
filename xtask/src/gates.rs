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
use std::thread;

use crate::report::{self, Violation, XtaskError};
use crate::{
    apisync, artifact, boundary, budget, color, depmap, docnum, guard, header, length, lexicon,
    modmap, npm, proof, release, render, secret, slices, specalign, wire_ts, wiring, wording,
};

/// How many gates run. The array below is typed by it, so the number and
/// the list are one token apart and cannot disagree; `vocabulary` reads
/// it so no document has to hold a copy.
pub(crate) const COUNT: usize = 23;

/// One gate: the name a person types, and the check it runs.
pub(crate) struct Gate {
    pub(crate) name: &'static str,
    check: fn(&Path, Option<&str>) -> Result<Vec<Violation>, XtaskError>,
}

/// Every gate, in the order a run reports them. The only roster: `--list`,
/// usage and name selection all read it.
pub(crate) const GATES: [Gate; COUNT] = [
    Gate {
        name: "header",
        check: |root, _| header::check(root),
    },
    Gate {
        name: "lexicon",
        check: |root, _| lexicon::check(root),
    },
    Gate {
        name: "modmap",
        check: |root, _| modmap::check(root),
    },
    Gate {
        name: "length",
        check: |root, _| length::check(root),
    },
    Gate {
        name: "boundary",
        check: |root, _| boundary::check(root),
    },
    // `slices` judges which code may name one path, so it walks sources
    // the way `boundary` walks them and sits beside it.
    Gate {
        name: "slices",
        check: |root, _| slices::check(root),
    },
    Gate {
        name: "artifact",
        check: |root, _| artifact::check(root),
    },
    Gate {
        name: "depmap",
        check: |root, _| depmap::check(root),
    },
    Gate {
        name: "npm",
        check: |root, _| npm::check(root),
    },
    Gate {
        name: "secret",
        check: |root, _| secret::check(root),
    },
    Gate {
        name: "color",
        check: |root, _| color::check(root),
    },
    Gate {
        name: "wording",
        check: |root, _| wording::check(root),
    },
    Gate {
        name: "render",
        check: |root, _| render::check(root),
    },
    Gate {
        name: "wiring",
        check: |root, _| wiring::check(root),
    },
    // `wire-ts` renders the client's wire types in this process and
    // compares one file; it sits beside `wiring` because both judge the
    // same socket seam.
    Gate {
        name: "wire-ts",
        check: |root, _| wire_ts::check(root),
    },
    Gate {
        name: "docnum",
        check: |root, _| docnum::check(root),
    },
    Gate {
        name: "proof",
        check: |root, _| proof::check(root),
    },
    Gate {
        name: "budget",
        check: |root, _| budget::check(root),
    },
    Gate {
        name: "specalign",
        check: |root, _| specalign::check(root),
    },
    // `features` runs the compiler rather than reading source; its
    // verdict is the default feature set's, test targets included, which
    // nothing else compiles.
    Gate {
        name: "features",
        check: |root, _| default_features(root),
    },
    Gate {
        name: "apisync",
        check: apisync::check,
    },
    Gate {
        name: "release",
        check: |root, _| release::check(root),
    },
    Gate {
        name: "guard",
        check: guard::check,
    },
];

/// The gates a run judges: all of them when `names` is empty, otherwise
/// exactly the named ones in roster order.
///
/// # Errors
/// [`XtaskError::UnknownGate`] for the first name the roster does not
/// hold, so a mistyped name refuses instead of running every gate.
pub(crate) fn select(names: &[String]) -> Result<Vec<&'static Gate>, XtaskError> {
    if let Some(unknown) = names
        .iter()
        .find(|name| !GATES.iter().any(|gate| gate.name == name.as_str()))
    {
        return Err(XtaskError::UnknownGate {
            name: unknown.clone(),
            known: GATES.map(|gate| gate.name).join(" "),
        });
    }
    Ok(GATES
        .iter()
        .filter(|gate| names.is_empty() || names.iter().any(|name| name == gate.name))
        .collect())
}

/// Run the gates `names` selects and report them together; the gates
/// judge in parallel, and the report keeps roster order.
pub(crate) fn run(root: &Path, range: Option<&str>, names: &[String]) -> ExitCode {
    let gates = match select(names) {
        Ok(gates) => gates,
        Err(err) => return report::internal_failure(&err),
    };
    report::finish_all(judge(&gates, root, range))
}

/// Every gate's verdict, in the order `gates` lists them. Each gate
/// judges on a thread of its own, so the wall time is the slowest gate's
/// rather than their sum; joining in roster order keeps the report
/// byte-identical between runs on one tree.
fn judge(
    gates: &[&'static Gate],
    root: &Path,
    range: Option<&str>,
) -> Vec<(&'static str, Result<Vec<Violation>, XtaskError>)> {
    thread::scope(|scope| {
        gates
            .iter()
            .map(|gate| (gate.name, scope.spawn(move || (gate.check)(root, range))))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(name, verdict)| {
                (
                    name,
                    verdict
                        .join()
                        .unwrap_or(Err(XtaskError::GatePanicked { name })),
                )
            })
            .collect()
    })
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use std::path::Path;
    use std::sync::Mutex;
    use std::thread::{self, ThreadId};

    use super::{Gate, judge, select};
    use crate::report::{Violation, XtaskError};

    static JUDGED_ON: Mutex<Vec<ThreadId>> = Mutex::new(Vec::new());

    fn record(_: &Path, _: Option<&str>) -> Result<Vec<Violation>, XtaskError> {
        JUDGED_ON.lock().unwrap().push(thread::current().id());
        Ok(Vec::new())
    }

    static FIRST: Gate = Gate {
        name: "first",
        check: record,
    };
    static SECOND: Gate = Gate {
        name: "second",
        check: record,
    };

    #[test]
    fn gates_judge_off_the_calling_thread_and_report_in_roster_order() {
        let names: Vec<&str> = judge(&[&FIRST, &SECOND], Path::new("."), None)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(names, ["first", "second"]);
        let caller = thread::current().id();
        let judged_on = JUDGED_ON.lock().unwrap().clone();
        assert_eq!(judged_on.len(), 2);
        assert!(
            judged_on.iter().all(|id| *id != caller),
            "a gate judged on the calling thread: {judged_on:?}"
        );
    }

    #[test]
    fn a_named_gate_runs_alone_and_an_unknown_name_is_refused() {
        let named: Vec<&str> = select(&["modmap".to_owned()])
            .unwrap()
            .iter()
            .map(|gate| gate.name)
            .collect();
        assert_eq!(named, ["modmap"]);
        assert!(matches!(
            select(&["modmpa".to_owned()]),
            Err(XtaskError::UnknownGate { name, .. }) if name == "modmpa"
        ));
    }
}
