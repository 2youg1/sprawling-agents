// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The conversation page's standing controls (tools/xtask/Spec.lean
//! §8-51, refrain P2 and P11): no fixed navigation or floating action
//! bar over the conversation, and the persistent entries of its three
//! clusters counted against the register.
//!
//! **A page is recognised by what a screen reader hears.** The
//! conversation region and the edge keys are landmarks named from
//! `lang.json`; the nearest measured box holding both is one conversation
//! page. The names are read in every language the phrase table has,
//! because the gallery is drawn in the language of the browser that
//! opens it.
//!
//! **The composer is found by its text box, not by its form.** The probe
//! measures regions, controls and anything with a role, and a `<form>` is
//! none of the three; the text box the conversation is written in is. A
//! control that neither scrolls with the thread nor answers a refusal
//! belongs to the composer when it reaches below the top of that box, and
//! is a fixed bar when it stands wholly above it.

use std::collections::BTreeMap;
use std::path::Path;

use browser::survey::{Drawn, Overflow};

use super::violation;
use crate::report::{Violation, XtaskError};
use crate::walk;

/// The phrase table the landmark names come from.
const LANG: &str = concat!(crate::walk::client_src!(), "/lang.json");

/// The register row the count is held to.
const ROW: &str = "talk_controls";

/// Roles whose controls P2 does not limit: a notice a person has to act
/// on in time, and a panel that was opened.
const RECOVERY_ROLES: [&str; 3] = ["alert", "status", "dialog"];

/// The landmark names the gate reads, each in every language.
pub(super) struct Words {
    conversation: Vec<String>,
    edge: Vec<String>,
    inspect: Vec<String>,
}

impl Words {
    /// The names `lang.json` gives the conversation region, the edge keys
    /// and the inspect side.
    ///
    /// # Errors
    /// `XtaskError::Doc` when the table does not parse or lacks one of the
    /// keys: a gate that cannot name what it looks for cannot judge.
    pub(super) fn read(root: &Path) -> Result<Self, XtaskError> {
        let text = walk::read_text(&root.join(LANG))?;
        let table: BTreeMap<String, BTreeMap<String, String>> = serde_json::from_str(&text)
            .map_err(|err| XtaskError::Doc {
                file: LANG.to_owned(),
                msg: format!("this file does not parse as a phrase table: {err}"),
            })?;
        let said = |key: &str| {
            table
                .get(key)
                .map(|languages| languages.values().cloned().collect())
                .ok_or_else(|| XtaskError::Doc {
                    file: LANG.to_owned(),
                    msg: format!(
                        "the render gate looks for the landmark `{key}`, and this table has no such phrase"
                    ),
                })
        };
        Ok(Self {
            conversation: said("region_conversation")?,
            edge: said("region_edge")?,
            inspect: said("region_inspect")?,
        })
    }
}

/// Judge every conversation page of one opening, and say how many
/// standing controls the most crowded one has; `None` when the opening
/// drew no conversation page at all.
pub(super) fn the_conversation_page_holds_its_controls(
    drawn: &[Drawn],
    at: &str,
    words: &Words,
    out: &mut Vec<Violation>,
) -> Option<u64> {
    let mut most: Option<u64> = None;
    for (page, region) in pages(drawn, words) {
        let Some(line) = text_box_top(drawn, region) else {
            out.push(violation(
                at,
                "a conversation page draws the box its words are written in",
                format!(
                    "{} holds no text box outside the thread",
                    called(drawn, region)
                ),
                "draw the composer inside the conversation region",
            ));
            continue;
        };
        let mut composer: u64 = 0;
        for held in standing(drawn, region) {
            if held.top.saturating_add(held.height) > line {
                composer = composer.saturating_add(1);
                continue;
            }
            out.push(violation(
                at,
                "the conversation has no fixed navigation and no floating action bar above its composer",
                format!(
                    "{} at x={} y={} neither scrolls with the thread nor belongs to the composer",
                    held.called(),
                    held.left,
                    held.top
                ),
                "put the control in a message, where it scrolls with the thread, or in the \
                 composer's row; a notice a person must act on in time is a `status` or an `alert`",
            ));
        }
        let counted = composer
            .saturating_add(controls_in_named(drawn, page, &words.edge))
            .saturating_add(controls_in_named(drawn, page, &words.inspect));
        most = Some(most.map_or(counted, |seen| seen.max(counted)));
    }
    most
}

/// Hold the count of one opening to the register: none drawn at all is a
/// gallery with nothing to judge, and a count above the accepted reading
/// plus its slack is a regression.
pub(super) fn talk_controls_within_register(
    counted: Option<u64>,
    register: &toml::Value,
    at: &str,
    out: &mut Vec<Violation>,
) {
    let Some(counted) = counted else {
        out.push(violation(
            at,
            "the gallery draws the conversation page whose standing controls this gate counts",
            "the gallery draws no conversation page".to_owned(),
            "keep a shell fixture that mounts the conversation region and the edge keys together",
        ));
        return;
    };
    let read = |key: &str| {
        register
            .get(ROW)
            .and_then(|row| row.get(key))
            .and_then(toml::Value::as_integer)
            .and_then(|value| u64::try_from(value).ok())
    };
    let (Some(best), Some(slack)) = (read("best_count"), read("slack_count")) else {
        out.push(violation(
            at,
            "the register states how many standing controls the conversation page was accepted with",
            format!("tools/xtask/budgets.toml has no whole `best_count` and `slack_count` in [{ROW}]"),
            "record the accepted fixture's reading in [talk_controls]",
        ));
        return;
    };
    if counted > best.saturating_add(slack) {
        out.push(violation(
            at,
            "the conversation page's standing controls may fall freely and grow only within the slack",
            format!("the most crowded conversation page stands {counted} controls; the register accepted {best} and {slack} more"),
            "fold the new control into a message, a fact's detail or an opened panel (refrain P11); \
             a control the page needs is recorded with its reason in [talk_controls]",
        ));
    }
}

/// Every conversation page: the nearest measured box holding both a
/// conversation region and the edge keys, with the region in it.
fn pages(drawn: &[Drawn], words: &Words) -> Vec<(usize, usize)> {
    let edges: Vec<usize> = named(drawn, &words.edge).collect();
    named(drawn, &words.conversation)
        .filter_map(|region| {
            parents(drawn, region)
                .find(|holder| edges.iter().any(|edge| holds(drawn, *holder, *edge)))
                .map(|page| (page, region))
        })
        .collect()
}

/// The regions whose accessible name is one of `names`: a landmark or a
/// named section. A word of the same spelling inside a region - the chat
/// mode is called 对话 too - is not a region.
fn named<'a>(drawn: &'a [Drawn], names: &'a [String]) -> impl Iterator<Item = usize> + 'a {
    drawn
        .iter()
        .enumerate()
        .filter(|(_, held)| {
            (held.landmark() || held.tag == "SECTION") && names.contains(&held.name)
        })
        .map(|(index, _)| index)
}

/// The parent chain of `index` as indices, nearest first.
fn parents(drawn: &[Drawn], index: usize) -> impl Iterator<Item = usize> + '_ {
    std::iter::successors(parent_of(drawn, index), move |up| parent_of(drawn, *up))
}

fn parent_of(drawn: &[Drawn], index: usize) -> Option<usize> {
    drawn
        .get(index)
        .and_then(|held| usize::try_from(held.parent).ok())
}

/// Whether the element at `holder` holds the element at `index`.
fn holds(drawn: &[Drawn], holder: usize, index: usize) -> bool {
    parents(drawn, index).any(|up| up == holder)
}

/// The top of the text box the conversation is written in: the first one
/// in the region that does not scroll with the thread.
fn text_box_top(drawn: &[Drawn], region: usize) -> Option<i64> {
    drawn
        .iter()
        .enumerate()
        .filter(|(index, held)| {
            (held.tag == "TEXTAREA" || held.role == "textbox")
                && held.drawn()
                && holds(drawn, region, *index)
                && !moves_or_answers(drawn, region, *index)
        })
        .map(|(_, held)| held.top)
        .next()
}

/// The controls in the region that stand still: drawn, operable, not
/// carried by something that scrolls, and not part of a notice or panel.
fn standing(drawn: &[Drawn], region: usize) -> impl Iterator<Item = &Drawn> {
    drawn.iter().enumerate().filter_map(move |(index, held)| {
        (held.operable()
            && held.drawn()
            && holds(drawn, region, index)
            && !moves_or_answers(drawn, region, index))
        .then_some(held)
    })
}

/// Whether something between `index` and the region scrolls (so the
/// control moves with the thread) or is a notice or a panel.
fn moves_or_answers(drawn: &[Drawn], region: usize, index: usize) -> bool {
    parents(drawn, index)
        .take_while(|up| *up != region)
        .filter_map(|up| drawn.get(up))
        .any(|up| {
            matches!(up.down, Overflow::Scrolls)
                || matches!(up.across, Overflow::Scrolls)
                || RECOVERY_ROLES.contains(&up.role.as_str())
        })
}

/// The operable controls drawn inside a landmark of the page named one
/// of `names`.
fn controls_in_named(drawn: &[Drawn], page: usize, names: &[String]) -> u64 {
    let clusters: Vec<usize> = named(drawn, names)
        .filter(|cluster| holds(drawn, page, *cluster))
        .collect();
    let inside = drawn.iter().enumerate().filter(|(index, held)| {
        held.operable()
            && held.drawn()
            && clusters
                .iter()
                .any(|cluster| holds(drawn, *cluster, *index))
    });
    u64::try_from(inside.count()).unwrap_or(u64::MAX)
}

fn called(drawn: &[Drawn], index: usize) -> String {
    drawn
        .get(index)
        .map_or_else(|| "a conversation region".to_owned(), Drawn::called)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
