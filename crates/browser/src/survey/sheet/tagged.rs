// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The same readings as named fields.
//!
//! One `<edit>` per repair, one `<at>` per place it has to be made.
//! `id` is the key an agent suppresses or diffs on and never changes
//! when a sentence is reworded; `fix` and `by` are the verb and the
//! operand, kept apart from the prose so that acting on them needs no
//! sentence parsing; `cohort` is how many of the population agreed,
//! which is what decides whether a repair is safe to make without
//! asking - eleven of twelve is a typing mistake and three of five is
//! a column that never had a consensus.

use std::collections::BTreeMap;

use super::super::{Deviation, Group};
use super::{Survey, remedy, rule, says};

pub(super) fn tagged(survey: Survey<'_>, census: Option<&str>) -> String {
    let mut edits: BTreeMap<(Group, String), Vec<&Deviation<'_>>> = BTreeMap::new();
    for one in survey.found {
        edits
            .entry((one.finding.group(), rule(one)))
            .or_default()
            .push(one);
    }
    let mut out = format!(
        "<survey page=\"{}\" edits=\"{}\" sites=\"{}\">\n",
        escaped(survey.at),
        edits.len(),
        survey.found.len()
    );
    for ((group, headline), sites) in edits {
        let Some(first) = sites.first() else { continue };
        out.push_str(&format!(
            "<edit id=\"{}\" group=\"{}\" standing=\"{}\" sites=\"{}\">\n",
            first.finding.id(),
            group.called(),
            match first.standing() {
                super::super::Standing::Refused => "refused",
                super::super::Standing::Noted => "noted",
            },
            sites.len()
        ));
        out.push_str(&format!("<why>{}</why>\n", escaped(&headline)));
        out.push_str(&format!("<fix>{}</fix>\n", escaped(&remedy(first))));
        for one in &sites {
            let where_ = survey
                .sources
                .locate(&one.at.class)
                .map_or_else(String::new, |found| format!(" src=\"{}\"", escaped(found)));
            out.push_str(&format!(
                "<site{where_} box=\"{}\" x=\"{}\" y=\"{}\">{}</site>\n",
                escaped(&one.at.tag),
                one.at.left,
                one.at.top,
                escaped(&says(one))
            ));
        }
        out.push_str("</edit>\n");
    }
    if let Some(line) = census {
        out.push_str(&format!("<unpainted>{}</unpainted>\n", escaped(line)));
    }
    out.push_str("</survey>\n");
    out
}

/// The five characters that would otherwise close a tag somebody is
/// still inside. A name on this page can hold any of them.
fn escaped(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
