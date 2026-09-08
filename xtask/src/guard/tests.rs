// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{gate_faces, is_protected, judged_faces, row_path, strikes_only_exemptions};

fn paths(list: &[&str]) -> Vec<String> {
    list.iter().map(|p| (*p).to_owned()).collect()
}

/// The shortcut this gate exists to close: the work and the rule
/// that would have refused it, in one commit and one green run.
#[test]
fn a_gate_loosened_beside_the_work_it_judges_is_the_shape_that_needs_a_ruling() {
    let mixed = paths(&["clippy.toml", "crates/kernel/src/plan.rs"]);
    assert!(!gate_faces(&mixed, false, false).is_empty());
    assert!(!judged_faces(&mixed).is_empty());
}

/// Re-pricing a rule in a commit of its own is ordinary work
/// (AGENTS.md, `guard` row). The old width charged a ruling for it,
/// and that price is the recorded mechanical reason a rule outlived
/// its argument.
#[test]
fn a_re_pricing_that_travels_alone_needs_no_ruling() {
    let alone = paths(&[
        "xtask/src/guard.rs",
        "xtask/xtask-SPEC.md",
        "AGENTS.md",
        "README.md",
    ]);
    assert!(
        !gate_faces(&alone, false, false).is_empty(),
        "still a gate change"
    );
    assert!(
        judged_faces(&alone).is_empty(),
        "nothing the gates judge rides along"
    );
}

/// A card that only changes product source never meets this gate,
/// whatever it does to the code.
#[test]
fn ordinary_source_work_is_not_a_gate_change() {
    let work = paths(&["crates/web/src/board.rs", "crates/web/screens/board.html"]);
    assert!(gate_faces(&work, false, false).is_empty());
}

/// What a gate produced is not how a gate decides: a public-surface
/// change regenerates a baseline beside the source it describes, and
/// the pair must not read as a loosening.
#[test]
fn a_regenerated_baseline_beside_its_own_source_is_not_a_gate_change() {
    let pair = paths(&[
        "xtask/api-baselines/kernel.txt",
        "crates/kernel/src/share.rs",
    ]);
    assert!(gate_faces(&pair, false, false).is_empty());
}

/// The length rule orders the strike in the same change as the
/// split, so charging a ruling for the pair would charge one for
/// every split that is left. A removal cannot loosen anything: the
/// file it names goes from its own pin to the budget everyone else
/// is held to.
#[test]
fn striking_an_exemption_beside_the_split_that_earned_it_is_not_a_gate_change() {
    let diff = "--- a/xtask/budgets.toml\n\
                +++ b/xtask/budgets.toml\n\
                @@ -246,7 +246,6 @@\n\
                 [file_length.predating]\n\
                -\"crates/memory/src/jsonl.rs\" = 1194\n\
                 \"crates/web/src/app.rs\" = 3916\n";
    assert!(strikes_only_exemptions(diff));
    let pair = paths(&["xtask/budgets.toml", "crates/memory/src/jsonl.rs"]);
    assert!(gate_faces(&pair, false, true).is_empty());
    assert!(
        !gate_faces(&pair, false, false).is_empty(),
        "the same paths without the strike shape stay guarded"
    );
}

/// Every other budgets.toml edit is judged as before. Raising a
/// budget is the loosening this gate exists for; adding a row is a
/// new exemption; and carrying an exemption to the address an item
/// moved to is an addition too, so a signature is fixed or left
/// alone rather than relocated with its excuse.
#[test]
fn a_widened_budget_or_a_new_exemption_is_still_a_gate_change() {
    let widened = "@@\n-budget_lines = 1000\n+budget_lines = 2000\n";
    assert!(!strikes_only_exemptions(widened));
    let added = "@@\n+\"crates/web/src/app.rs\" = 3916\n";
    assert!(!strikes_only_exemptions(added));
    let moved = "@@\n\
                 -\"crates/sprawling/src/assembly.rs::conclude\", # 10\n\
                 +\"crates/sprawling/src/serving.rs::conclude\", # 10\n";
    assert!(!strikes_only_exemptions(moved));
    let comment = "@@\n-# the eight files that predate the rule\n";
    assert!(!strikes_only_exemptions(comment));
    let nothing = "@@\n context only\n";
    assert!(!strikes_only_exemptions(nothing), "a strike must be there");
}

/// A refusal that lists eighty paths is a refusal nobody reads.
#[test]
fn the_judged_side_of_a_refusal_is_bounded() {
    let many = paths(&[
        "crates/a/src/one.rs",
        "crates/a/src/two.rs",
        "crates/a/src/three.rs",
        "crates/a/src/four.rs",
        "crates/a/src/five.rs",
    ]);
    let named: Vec<String> = judged_faces(&many);
    assert_eq!(named.len(), 4);
    assert_eq!(named.last().map(String::as_str), Some("and others"));
}

#[test]
fn status_flip_is_not_a_row_removal() {
    // A flipped status deletes and re-adds the same path.
    let removed = row_path("| kernel::gate | crates/kernel/src/gate.rs | x | 8.2 | S2 | 未建 |");
    let added = row_path("| kernel::gate | crates/kernel/src/gate.rs | x | 8.2 | S2 | 已建 |");
    assert_eq!(removed, added);
    assert!(removed.is_some());
    // Prose mentioning crates/ is not a row.
    assert_eq!(row_path(" see crates/kernel/src/gate.rs "), None);
}

#[test]
fn protection_matches_roots_and_prefixes_only() {
    assert!(is_protected("deny.toml"));
    assert!(is_protected("Cargo.toml"));
    assert!(is_protected("xtask/src/guard.rs"));
    assert!(is_protected(".github/workflows/ci.yml"));
    // member manifests are not the root manifest
    assert!(!is_protected("crates/kernel/Cargo.toml"));
    assert!(!is_protected("ARCHITECTURE.md"));
    // What a gate produced is not how a gate decides. `apisync`
    // orders this file to be regenerated whenever a public surface
    // moves; guarding it would make the two gates contradict, and a
    // ruling demanded on every public-surface change is a ruling
    // nobody reads.
    assert!(!is_protected("xtask/api-baselines/channels.txt"));
    assert!(
        is_protected("xtask/src/apisync.rs"),
        "the gate's own machinery stays guarded"
    );
}
