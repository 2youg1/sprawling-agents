// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::Path;

use browser::survey::{Drawn, Overflow};

use super::violation;
use crate::report::{Violation, XtaskError};
use crate::walk;

const LANG: &str = concat!(crate::walk::client_src!(), "/lang.json");

const ROW: &str = "talk_controls";

const RECOVERY_ROLES: [&str; 3] = ["alert", "status", "dialog"];

pub(super) struct Words {
    conversation: Vec<String>,
    edge: Vec<String>,
    inspect: Vec<String>,
}

impl Words {

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
        let mut headed: u64 = 0;
        for (index, held) in standing(drawn, region) {
            if held.top.saturating_add(held.height) > line {
                composer = composer.saturating_add(1);
                continue;
            }
            if headed == 0 && in_head(drawn, region, index) {
                headed = 1;
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
            format!("Tools/xtask/budgets.toml has no whole `best_count` and `slack_count` in [{ROW}]"),
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

fn named<'a>(drawn: &'a [Drawn], names: &'a [String]) -> impl Iterator<Item = usize> + 'a {
    drawn
        .iter()
        .enumerate()
        .filter(|(_, held)| {
            (held.landmark() || held.tag == "SECTION") && names.contains(&held.name)
        })
        .map(|(index, _)| index)
}

fn parents(drawn: &[Drawn], index: usize) -> impl Iterator<Item = usize> + '_ {
    std::iter::successors(parent_of(drawn, index), move |up| parent_of(drawn, *up))
}

fn parent_of(drawn: &[Drawn], index: usize) -> Option<usize> {
    drawn
        .get(index)
        .and_then(|held| usize::try_from(held.parent).ok())
}

fn holds(drawn: &[Drawn], holder: usize, index: usize) -> bool {
    parents(drawn, index).any(|up| up == holder)
}

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

fn standing(drawn: &[Drawn], region: usize) -> impl Iterator<Item = (usize, &Drawn)> {
    drawn.iter().enumerate().filter_map(move |(index, held)| {
        (held.operable()
            && held.drawn()
            && holds(drawn, region, index)
            && !moves_or_answers(drawn, region, index))
        .then_some((index, held))
    })
}

fn in_head(drawn: &[Drawn], region: usize, index: usize) -> bool {
    parents(drawn, index)
        .take_while(|up| *up != region)
        .filter_map(|up| drawn.get(up))
        .any(|up| up.tag == "HEADER")
}

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
