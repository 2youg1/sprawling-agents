// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Readability: the seventh token assertion, which pairs every text token
//! with the brightest surface text may sit on and every type step with the
//! tier its size and weight demand (tools/xtask/Spec.lean §8-8).

use super::contrast::{apca_lc, bronze_tier};
use super::tables::{parse_text_tokens, parse_type_scale, text_surface_ceiling, tier_slack};
use super::{Mode, token_violation};
use crate::report::Violation;

/// The seventh assertion: every text token reaches the tier it claims, and
/// every type step claims the tier its size and weight actually demand.
///
/// This one is here rather than in a browser because it never needed one.
/// The surfaces are a closed ladder and the tokens are a closed table, so
/// the pairs are enumerable and the judgement is a pure function - what the
/// architecture calls a missing end-to-end gate is, for contrast, a missing
/// table. It is checked against `TEXT_SURFACE_CEILING`, the brightest
/// surface text may sit on, because a token that passed on the page and
/// failed on a card would be one rule with two answers.
pub(super) fn judge_readability(
    source: &str,
    greys: &[(String, u16)],
    mode: Mode,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let named = |rule: &str, violation: String| {
        token_violation(rule, format!("{}: {violation}", mode.name()))
    };
    let tokens = parse_text_tokens(source);
    let steps = parse_type_scale(source);
    if tokens.is_empty() || steps.is_empty() {
        violations.push(named(
            "the text token and type tables are readable",
            "TEXT_TOKENS or TYPE_SCALE parsed to nothing".to_owned(),
        ));
        return violations;
    }
    let Some(surface) = text_surface_ceiling(source).and_then(|name| {
        greys
            .iter()
            .find(|(rung, _)| *rung == name)
            .map(|(_, l)| *l)
    }) else {
        violations.push(named(
            "the surface furthest from the page that carries text is a rung of the ramp",
            "TEXT_SURFACE_CEILING names no rung of GRAY_RAMP".to_owned(),
        ));
        return violations;
    };

    let Some(slack) = tier_slack(source) else {
        violations.push(named(
            "the rounding a tier allows is declared",
            "`--tier-slack` is not declared as a number from 0".to_owned(),
        ));
        return violations;
    };

    for (name, lightness, claimed) in &tokens {
        let reached = apca_lc(*lightness, surface);
        if reached + slack < f64::from(*claimed) {
            violations.push(named(
                "a text token reaches the tier it claims",
                format!("{name} claims Lc {claimed} and reaches {reached:.1}"),
            ));
        }
    }

    for (name, px, weight, claimed) in &steps {
        // A step called `body` is prose and takes the body column; every
        // other step is something that qualifies prose and takes the
        // content column. Bronze states different minimum sizes for the
        // two, and merging them would let a 15px note claim a tier only a
        // 15px sentence may claim.
        let derived = bronze_tier(*px, *weight, name == "body");
        match derived {
            None => violations.push(token_violation(
                "every type step is large enough for some tier to admit it",
                format!(
                    "{name} is {px}px at weight {weight}, below every Bronze minimum: \
                     no colour makes it legible"
                ),
            )),
            Some(tier) if tier != *claimed => violations.push(token_violation(
                "a type step claims the tier its size and weight demand",
                format!("{name} claims Lc {claimed} and demands Lc {tier}"),
            )),
            Some(_) => {}
        }
        if let Some(tier) = derived
            && !tokens.iter().any(|(_, _, claim)| *claim >= tier)
        {
            violations.push(token_violation(
                "some text token can serve every type step",
                format!("{name} demands Lc {tier} and no text token reaches it"),
            ));
        }
    }
    violations
}
