// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The domain doors: whether a run may write one file, reach a declared
//! area, or change a file already there, each read from its WriteDomain.
//! The part `crates/kernel/spec/Gate.lean` specifies this module with the
//! rest of `kernel::gate`.

use crate::address::Address;
use crate::error::{AxCode, AxError, GateRefusal};
use crate::taint::TaintSet;
use crate::write_domain::{DocumentReason, DomainVerdict, WriteDomain, WriteLimit};

use super::GateOutcome;

/// Whether one file may be written: inside the prefixes, and the kind
/// of file this domain writes.
pub fn domain(domain: &WriteDomain, target: &Address, taint: &TaintSet) -> GateOutcome {
    match domain.admits(target) {
        DomainVerdict::Within => GateOutcome::Allow,
        DomainVerdict::NotWritable { reason } => not_writable(target, reason),
        DomainVerdict::Outside { prefixes } => outside(target, prefixes, taint),
    }
}

/// Whether a declared area is reachable at all. A tool declares the
/// room it works in, not a file, so this asks only the prefix question:
/// a documents domain asked whether `hall/mayor` is Markdown would say
/// no, and the Mayor could write nothing.
pub fn reach(domain: &WriteDomain, area: &Address, taint: &TaintSet) -> GateOutcome {
    if domain.reaches(area) {
        GateOutcome::Allow
    } else {
        outside(
            area,
            domain.prefixes().map(|p| p.as_str().to_owned()).collect(),
            taint,
        )
    }
}

/// Whether a write that may change a file already there is open under
/// this run's write limit (`crates/kernel/spec/WriteDomain.lean` §8-78).
///
/// Asked by every write path before it touches a file that exists, or
/// before it starts anything that could: the edit tool's replacing arm,
/// and a command placed on the host. Whether the target exists is not
/// asked here; a create that finds its name taken is refused by the
/// filesystem at the moment of the write, under either limit.
#[must_use]
pub fn replacing(limit: WriteLimit, target: &Address) -> GateOutcome {
    match limit {
        WriteLimit::Full => GateOutcome::Allow,
        WriteLimit::Create => GateOutcome::Deny {
            refusal: Box::new(
                AxError::refusal(
                    AxCode::OutsideWriteDomain,
                    "write file",
                    target.as_str(),
                    GateRefusal::new(
                        "this run creates files and changes none",
                        format!(
                            "{} would change what is already there, and this run was \
                             dispatched under the create write limit",
                            target.as_str()
                        ),
                        "write what you have to the path of a file that does not exist yet",
                    ),
                )
                .with_recovery(
                    "the write limit is chosen when the work is dispatched; ask the person to \
                     dispatch it again with the full write limit to change existing files",
                ),
            ),
        },
    }
}

/// The one refusal for a target beyond the prefixes, whether it is a
/// file or an area.
fn outside(target: &Address, prefixes: Vec<String>, taint: &TaintSet) -> GateOutcome {
    let mut violation = format!("target {} is outside the write domain", target.as_str());
    if !taint.is_empty() {
        violation.push_str(&format!("; the action carries content from {taint}"));
    }
    let alternative = if prefixes.is_empty() {
        "this actor writes nowhere; read, or hand the change to an actor with a domain".to_owned()
    } else {
        format!("write under one of: {}", prefixes.join(", "))
    };
    GateOutcome::Deny {
        refusal: Box::new(
            AxError::refusal(
                AxCode::OutsideWriteDomain,
                "write file",
                target.as_str(),
                GateRefusal::new(
                    "writes land inside the write domain",
                    violation,
                    alternative,
                ),
            )
            .with_nearby(prefixes)
            .with_recovery(
                "a write domain widens in one place only, the building's \
                 `.sprawling/RULES.toml`, and the `rules` tool is how you propose that edit",
            ),
        ),
    }
}

/// The refusal a documents domain owes a target inside its own
/// prefixes: the target is reachable, and this actor still does not
/// write that kind of file.
fn not_writable(target: &Address, reason: DocumentReason) -> GateOutcome {
    let (violation, alternative) = match reason {
        DocumentReason::NotMarkdown => (
            format!("{} is not a Markdown document", target.as_str()),
            "this actor writes documents; put what you have to say in a `.md` file, or hand \
             the change to a resident whose domain takes code"
                .to_owned(),
        ),
        DocumentReason::ThePlan => (
            format!(
                "{} is a plan file, and no write domain reaches one",
                target.as_str()
            ),
            format!(
                "use the `plan` tool: it is the one editing entrance to {}, and it keeps the \
                 six-column table readable",
                crate::spine::ROADMAP_FILE
            ),
        ),
    };
    GateOutcome::Deny {
        refusal: Box::new(
            AxError::refusal(
                AxCode::OutsideWriteDomain,
                "write file",
                target.as_str(),
                GateRefusal::new(
                    "this write domain writes Markdown documents and never a plan",
                    violation,
                    alternative,
                ),
            )
            .with_recovery(
                "the kinds of file this domain takes are declared in the building's \
                 `.sprawling/RULES.toml`; propose a change to it through the `rules` tool",
            ),
        ),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::taint::TaintSource;

    fn assert_three_parts(outcome: &GateOutcome, code: AxCode) {
        match outcome {
            GateOutcome::Deny { refusal } => {
                assert_eq!(refusal.code(), &code);
                let gate = refusal.gate().expect("gate refusals carry three parts");
                assert!(!gate.rule().is_empty());
                assert!(!gate.violation().is_empty());
                assert!(!gate.alternative().is_empty());
            }
            _ => panic!("expected a refusal"),
        }
    }

    #[test]
    fn the_domain_door_refuses_with_prefixes_as_nearby() {
        let wd = WriteDomain::new(vec![Address::parse("b1/room").unwrap()]).unwrap();
        let inside = Address::parse("b1/room/notes.md").unwrap();
        assert!(matches!(
            domain(&wd, &inside, &TaintSet::empty()),
            GateOutcome::Allow
        ));
        let outside = Address::parse("b2/other.md").unwrap();
        let outcome = domain(&wd, &outside, &TaintSet::empty());
        assert_three_parts(&outcome, AxCode::OutsideWriteDomain);
        let GateOutcome::Deny { refusal } = outcome else {
            panic!("refusal shape asserted above")
        };
        assert_eq!(refusal.nearby(), ["b1/room"]);
        // Tainted violations name their provenance in the violation part.
        let tainted = TaintSet::of(TaintSource::new("web:evil").unwrap());
        let outcome = domain(&wd, &outside, &tainted);
        let GateOutcome::Deny { refusal } = outcome else {
            panic!("refusal shape asserted above")
        };
        assert!(refusal.gate().unwrap().violation().contains("web:evil"));
    }

    /// The full limit adds nothing to the domain; the create limit
    /// refuses every write that would change a file already there, with
    /// the target as the subject and the way to a new file as the
    /// alternative.
    #[test]
    fn the_create_limit_refuses_a_replacing_write_and_the_full_one_does_not() {
        let existing = Address::parse("b1/room/notes.md").unwrap();
        assert!(matches!(
            replacing(WriteLimit::Full, &existing),
            GateOutcome::Allow
        ));
        let outcome = replacing(WriteLimit::Create, &existing);
        assert_three_parts(&outcome, AxCode::OutsideWriteDomain);
        let GateOutcome::Deny { refusal } = outcome else {
            panic!("refusal shape asserted above")
        };
        assert_eq!(refusal.subject(), "b1/room/notes.md");
        assert!(
            refusal
                .gate()
                .unwrap()
                .alternative()
                .contains("does not exist")
        );
    }

    /// City Hall's residents write Markdown and nothing else, and the
    /// plan file is refused to them at whatever depth it sits: the
    /// `plan` tool is its one editing entrance.
    #[test]
    fn a_documents_domain_takes_markdown_refuses_code_and_never_the_plan() {
        let wd = WriteDomain::documents(vec![Address::parse("hall").unwrap()]).unwrap();
        let empty = TaintSet::empty();
        assert!(matches!(
            domain(&wd, &Address::parse("hall/Memo.md").unwrap(), &empty),
            GateOutcome::Allow
        ));

        let code = domain(&wd, &Address::parse("hall/build.rs").unwrap(), &empty);
        assert_three_parts(&code, AxCode::OutsideWriteDomain);
        let GateOutcome::Deny { refusal } = code else {
            panic!("refusal shape asserted above")
        };
        assert!(
            refusal
                .gate()
                .unwrap()
                .violation()
                .contains("not a Markdown document")
        );

        for plan in ["hall/Roadmap.md", "hall/mayor/Roadmap.md"] {
            let outcome = domain(&wd, &Address::parse(plan).unwrap(), &empty);
            assert_three_parts(&outcome, AxCode::OutsideWriteDomain);
            let GateOutcome::Deny { refusal } = outcome else {
                panic!("refusal shape asserted above")
            };
            assert!(
                refusal
                    .gate()
                    .unwrap()
                    .alternative()
                    .contains("`plan` tool"),
                "{plan} is refused with the door that does reach it"
            );
        }

        // The reserved subtree is still Outside rather than
        // NotWritable: it is not this actor's ground at all.
        assert!(matches!(
            wd.admits(&Address::parse("hall/.sprawling/MAYOR.md").unwrap()),
            crate::write_domain::DomainVerdict::Outside { .. }
        ));
    }
}
