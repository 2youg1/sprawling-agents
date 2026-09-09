// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::capabilities::CITY_CAPABILITIES;
use super::{Ablation, Capability, Cost};

/// The document every resident reads first, ablated by the run below.
/// Read here rather than in the decision so the decision stays a
/// function of the text it is given.
const CITY_MD: &str = include_str!("../../../../docs/City.md");

const TWO: [Capability; 2] = [
    Capability {
        name: "ask for the time",
        cue: "Call `status`",
    },
    Capability {
        name: "verify a claim",
        cue: "is a claim",
    },
];

fn document() -> String {
    [
        "A first paragraph that grants nothing.",
        "",
        "This prompt does not hold the time. Call `status` for it.",
        "- A result from another agent is a claim.",
        "",
        "A closing paragraph, where a result is a claim once more.",
    ]
    .join("\n")
}

#[test]
fn a_bullet_list_belongs_to_the_paragraph_above_it() {
    let ablation = Ablation::new(&document(), &TWO).unwrap();
    assert_eq!(ablation.charges().len(), 3);
}

#[test]
fn a_passage_that_grants_nothing_is_untouched() {
    let ablation = Ablation::new(&document(), &TWO).unwrap();
    let charges = ablation.charges();
    assert_eq!(charges[0].cost, Cost::Untouched);
}

#[test]
fn the_only_place_a_capability_is_said_is_the_one_that_costs() {
    let ablation = Ablation::new(&document(), &TWO).unwrap();
    let charges = ablation.charges();
    assert_eq!(
        charges[1].cost,
        Cost::Sole {
            lost: vec!["ask for the time"],
        }
    );
}

#[test]
fn a_capability_said_twice_survives_the_loss_of_one_place() {
    let ablation = Ablation::new(&document(), &TWO).unwrap();
    let charges = ablation.charges();
    assert_eq!(
        charges[2].cost,
        Cost::Restated {
            also_said: vec!["verify a claim"],
        }
    );
}

#[test]
fn a_cue_the_document_never_says_is_refused_at_construction() {
    const ABSENT: [Capability; 1] = [Capability {
        name: "read the roadmap",
        cue: "Roadmap.md",
    }];
    let refusal = Ablation::new(&document(), &ABSENT).unwrap_err();
    assert_eq!(refusal.code(), &kernel::AxCode::InvalidArgs);
}

#[test]
fn the_costliest_passage_is_first_and_a_tie_goes_to_the_shorter_one() {
    let text = [
        "Call `status` for the time.",
        "",
        "A result from another agent is a claim, and this passage is much longer than the one above it.",
    ]
    .join("\n");
    let ablation = Ablation::new(&text, &TWO).unwrap();
    let ranked = ablation.costliest_first();
    assert_eq!(ranked[0].passage.index, 0);
    assert_eq!(ranked[1].passage.index, 1);
}

/// The run this card exists for. Ignored on purpose: it is evidence for
/// the next edit of `docs/City.md`, not a wall in front of it.
///
/// ```text
/// cargo nextest run -p eval --run-ignored all -E 'test(city_md)' --no-capture
/// ```
#[test]
#[ignore = "on demand: evidence for the next edit of City.md, never a gate"]
fn city_md_ablation_run() {
    let ablation = Ablation::new(CITY_MD, CITY_CAPABILITIES).unwrap();
    println!("City.md ablation: {} passages", ablation.charges().len());
    for charge in ablation.costliest_first() {
        let cost = match &charge.cost {
            Cost::Untouched => "-".to_owned(),
            Cost::Restated { also_said } => format!("restated: {}", also_said.join(", ")),
            Cost::Sole { lost } => format!("{} lost: {}", lost.len(), lost.join(", ")),
        };
        println!(
            "  [{}] {} bytes | {} | {}",
            charge.passage.index,
            charge.passage.removed.get(),
            charge.passage.opening,
            cost
        );
    }
}
