// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::address::Address;
use crate::error::{AxCode, AxError, GateRefusal};
use crate::taint::TaintSet;
use crate::write_domain::{DocumentReason, DomainVerdict, WriteDomain, WriteLimit};

use super::GateOutcome;

pub fn domain(domain: &WriteDomain, target: &Address, taint: &TaintSet) -> GateOutcome {
    match domain.admits(target) {
        DomainVerdict::Within => GateOutcome::Allow,
        DomainVerdict::NotWritable { reason } => not_writable(target, reason),
        DomainVerdict::Outside { prefixes } => outside(target, prefixes, taint),
    }
}

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
                    "the write limit is chosen when the work is dispatched; ask the User to \
                     dispatch it again with the full write limit to change existing files",
                ),
            ),
        },
    }
}

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
                 `.sprawling/RULES.toml`, which the User edits; ask the User for that change",
            ),
        ),
    }
}

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
                 `.sprawling/RULES.toml`, which the User edits; ask the User to change it",
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

        let tainted = TaintSet::of(TaintSource::new("web:evil").unwrap());
        let outcome = domain(&wd, &outside, &tainted);
        let GateOutcome::Deny { refusal } = outcome else {
            panic!("refusal shape asserted above")
        };
        assert!(refusal.gate().unwrap().violation().contains("web:evil"));
    }

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

        assert!(matches!(
            wd.admits(&Address::parse("hall/.sprawling/MAYOR.md").unwrap()),
            crate::write_domain::DomainVerdict::Outside { .. }
        ));
    }
}
