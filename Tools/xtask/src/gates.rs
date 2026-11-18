// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Run every gate in roster order and aggregate findings. `GATES`/`COUNT`
//! are the only authority for which gates run; `report` owns the exit.

use std::path::Path;
use std::process::ExitCode;
use std::thread;

use crate::report::{self, Violation, XtaskError};
use crate::{
    artifact, boundary, budget, color, comments, depmap, docnum, guard, header, length, lexicon,
    modmap, motion, npm, packaged, proof, release, render, residue, secret, spec, specalign,
    unused, upstream, wire_ts, wiring, wording,
};

/// Roster length; types `GATES` so the count and the list cannot disagree.
pub(crate) const COUNT: usize = 27;

/// One gate: the name a person types, and the check it runs.
pub(crate) struct Gate {
    pub(crate) name: &'static str,
    check: fn(&Path) -> Result<Vec<Violation>, XtaskError>,
}

/// Sole roster: `--list`, usage and name selection all read it.
pub(crate) const GATES: [Gate; COUNT] = [
    Gate {
        name: "header",
        check: header::check,
    },
    Gate {
        name: "lexicon",
        check: lexicon::check,
    },
    Gate {
        name: "modmap",
        check: modmap::check,
    },
    Gate {
        name: "length",
        check: length::check,
    },
    Gate {
        name: "comments",
        check: comments::check,
    },
    Gate {
        name: "boundary",
        check: boundary::check,
    },
    Gate {
        name: "artifact",
        check: artifact::check,
    },
    Gate {
        name: "packaged",
        check: packaged::check,
    },
    Gate {
        name: "depmap",
        check: depmap::check,
    },
    Gate {
        name: "unused",
        check: unused::check,
    },
    Gate {
        name: "npm",
        check: npm::check,
    },
    Gate {
        name: "secret",
        check: secret::check,
    },
    Gate {
        name: "color",
        check: color::check,
    },
    Gate {
        name: "motion",
        check: motion::check,
    },
    Gate {
        name: "wording",
        check: wording::check,
    },
    Gate {
        name: "render",
        check: render::check,
    },
    Gate {
        name: "wiring",
        check: wiring::check,
    },
    Gate {
        name: "wire-ts",
        check: wire_ts::check,
    },
    Gate {
        name: "docnum",
        check: docnum::check,
    },
    Gate {
        name: "upstream",
        check: upstream::check,
    },
    Gate {
        name: "proof",
        check: proof::check,
    },
    Gate {
        name: "budget",
        check: budget::check,
    },
    Gate {
        name: "specalign",
        check: specalign::check,
    },
    Gate {
        name: "spec",
        check: spec::check,
    },
    Gate {
        name: "release",
        check: release::check,
    },
    Gate {
        name: "guard",
        check: guard::check,
    },
    Gate {
        name: "residue",
        check: residue::check,
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
pub(crate) fn run(root: &Path, names: &[String], files: Option<Vec<String>>) -> ExitCode {
    let gates = match select(names) {
        Ok(gates) => gates,
        Err(err) => return report::internal_failure(&err),
    };
    let scope = match files
        .map(|paths| crate::source::Scope::new(&paths))
        .transpose()
    {
        Ok(scope) => scope,
        Err(err) => return report::internal_failure(&err),
    };
    if scope.is_some()
        && let Err(err) = ensure_scope_supported(&gates)
    {
        return report::internal_failure(&err);
    }
    report::finish_all(judge(&gates, root, scope.as_ref()))
}

fn ensure_scope_supported(gates: &[&Gate]) -> Result<(), XtaskError> {
    if let Some(gate) = gates
        .iter()
        .find(|gate| !matches!(gate.name, "comments" | "length"))
    {
        return Err(XtaskError::UnsupportedFileScope { name: gate.name });
    }
    Ok(())
}

/// Every gate's verdict, in the order `gates` lists them. Each gate
/// judges on a thread of its own, so the wall time is the slowest gate's
/// rather than their sum; joining in roster order keeps the report
/// byte-identical between runs on one tree.
fn judge(
    gates: &[&'static Gate],
    root: &Path,
    file_scope: Option<&crate::source::Scope>,
) -> Vec<(&'static str, Result<Vec<Violation>, XtaskError>)> {
    thread::scope(|scope| {
        gates
            .iter()
            .map(|gate| {
                (
                    gate.name,
                    scope.spawn(move || match (gate.name, file_scope) {
                        ("comments", Some(scope)) => comments::check_scoped(root, scope),
                        ("length", Some(scope)) => length::check_scoped(root, scope),
                        _ => (gate.check)(root),
                    }),
                )
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(name, verdict)| {
                (
                    name,
                    verdict.join().unwrap_or_else(|payload| {
                        Err(XtaskError::GatePanicked {
                            name,
                            message: panic_message(payload.as_ref()),
                        })
                    }),
                )
            })
            .collect()
    })
}

/// The text a panic carried: `panic!` with a literal leaves a `&str`,
/// with format arguments a `String`; any other payload has no words.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a payload that is not text".to_owned())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use std::path::Path;
    use std::sync::Mutex;
    use std::thread::{self, ThreadId};

    use super::{Gate, ensure_scope_supported, judge, select};
    use crate::report::{Violation, XtaskError};

    static JUDGED_ON: Mutex<Vec<ThreadId>> = Mutex::new(Vec::new());

    fn record(_: &Path) -> Result<Vec<Violation>, XtaskError> {
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

    fn explode(_: &Path) -> Result<Vec<Violation>, XtaskError> {
        panic!("the fixture could not be read")
    }

    static EXPLODES: Gate = Gate {
        name: "explodes",
        check: explode,
    };

    #[test]
    fn a_gate_that_panics_reports_the_panic_message_in_its_verdict() {
        let verdicts: Vec<(&str, String)> = judge(&[&EXPLODES], Path::new("."), None)
            .into_iter()
            .map(|(name, verdict)| (name, verdict.unwrap_err().to_string()))
            .collect();
        assert_eq!(
            verdicts,
            [(
                "explodes",
                "run gate `explodes`: the gate panicked with \"the fixture could not be read\" (gate-panicked); run `cargo xtask gates explodes` to see the panic alone".to_owned()
            )]
        );
    }

    #[test]
    fn gates_judge_off_the_calling_thread_and_report_in_roster_order() {
        JUDGED_ON.lock().unwrap().clear();
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
    fn file_scope_refuses_a_gate_without_a_scoped_judgement() {
        assert!(ensure_scope_supported(&[&FIRST]).is_err());
        let comments = Gate {
            name: "comments",
            check: record,
        };
        let length = Gate {
            name: "length",
            check: record,
        };
        assert!(ensure_scope_supported(&[&comments, &length]).is_ok());
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
