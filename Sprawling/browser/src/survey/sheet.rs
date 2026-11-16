// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

mod tagged;

use super::{Deviation, Finding, Group, Near, Page, Population, Sources, vocabulary};

const LISTED: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Prose,

    Tagged,
}

#[derive(Clone, Copy)]
pub struct Survey<'a> {
    pub page: &'a Page,
    pub at: &'a str,
    pub found: &'a [Deviation<'a>],
    pub sources: &'a Sources,
}

pub fn written(survey: Survey<'_>, shape: Shape) -> String {
    let census = vocabulary::unpainted(survey.page);
    if survey.found.is_empty() && census.is_none() {
        return String::new();
    }
    if shape == Shape::Tagged {
        return tagged::tagged(survey, census.as_deref());
    }
    let mut groups: BTreeMap<Group, Vec<&Deviation<'_>>> = BTreeMap::new();
    for one in survey.found {
        groups.entry(one.finding.group()).or_default().push(one);
    }
    let mut out = format!(
        "survey \u{b7} {} \u{b7} {} group(s) \u{b7} {} finding(s)\nfix in order; each group moves boxes and \
         invalidates the next\n",
        survey.at,
        groups.len(),
        survey.found.len()
    );
    for group in Group::IN_ORDER {
        let Some(members) = groups.get(&group) else {
            continue;
        };
        out.push_str(&written_group(group, members, survey.sources));
    }
    if let Some(line) = census {
        out.push_str(&format!("\n## vocabulary\n{line}\n"));
    }
    out
}

fn written_group(group: Group, members: &[&Deviation<'_>], sources: &Sources) -> String {
    let mut edits: BTreeMap<String, Vec<&Deviation<'_>>> = BTreeMap::new();
    for one in members {
        edits.entry(rule(one)).or_default().push(one);
    }
    let mut out = format!(
        "\n## {} · {} edit(s) · {} site(s)\n",
        group.called(),
        edits.len(),
        members.len()
    );
    for (headline, sites) in edits {
        out.push_str(&format!("{headline}\n"));
        for one in sites.iter().take(LISTED) {
            out.push_str(&format!("  {}\n", site(one, sources)));
        }
        if let Some(rest) = sites.len().checked_sub(LISTED)
            && rest > 0
        {
            out.push_str(&format!("  … {rest} more identical\n"));
        }
        if let Some(first) = sites.first() {
            out.push_str(&format!("  → {}\n", remedy(first)));
        }
    }
    out
}

pub fn rule(one: &Deviation<'_>) -> String {
    match one.finding {
        Finding::OutOfStep { among, .. } => among_says(among),
        Finding::Escapes { .. } => "nothing is drawn outside the box that holds it".to_owned(),
        Finding::TextCut { .. } => {
            "no box cuts its own text without drawing a mark where it cut".to_owned()
        }
        Finding::UndeclaredPaint {
            painted,
            on,
            ref nearest,
        } => format!(
            "{painted} is {} of a box and no word this page declares{}",
            on.of(),
            nearby(nearest.as_ref())
        ),
        Finding::UndeclaredSize {
            px_x100,
            ref nearest,
        } => format!(
            "{} is a type size no step this page declares{}",
            pixels(px_x100),
            nearby(nearest.as_ref())
        ),
        Finding::Echoed { .. } => {
            "one fact is painted in two places, and nothing links them".to_owned()
        }
    }
}

pub fn says(one: &Deviation<'_>) -> String {
    let at = one.at;
    match one.finding {
        Finding::OutOfStep {
            among,
            is,
            should_be,
            cohort,
            ..
        } => format!(
            "the {} of {} at x={} y={} reads {is}; {} of {} read {should_be}",
            among_axis(among),
            at.called(),
            at.left,
            at.top,
            cohort.with,
            cohort.of
        ),
        Finding::Escapes { holder, side } => format!(
            "{} ({},{} {}x{}) crosses the {} of {} ({},{} {}x{})",
            at.called(),
            at.left,
            at.top,
            at.width,
            at.height,
            side.of(),
            holder.called(),
            holder.left,
            holder.top,
            holder.width,
            holder.height
        ),
        Finding::TextCut { shown, needs } => format!(
            "{} at x={} y={} shows {shown}px of {needs}px of its own text",
            at.called(),
            at.left,
            at.top
        ),
        Finding::UndeclaredPaint { painted, on, .. } => format!(
            "{} at x={} y={} takes {painted} as {}",
            at.called(),
            at.left,
            at.top,
            on.of()
        ),
        Finding::UndeclaredSize { px_x100, .. } => format!(
            "{} at x={} y={} sets its text at {}",
            at.called(),
            at.left,
            at.top,
            pixels(px_x100)
        ),
        Finding::Echoed { other, text, homes } => format!(
            "`{text}` is painted by {} at x={} y={} and by {} at x={} y={}, in {homes} places",
            at.called(),
            at.left,
            at.top,
            other.called(),
            other.left,
            other.top
        ),
    }
}

pub fn remedy(one: &Deviation<'_>) -> String {
    match one.finding {
        Finding::OutOfStep {
            among,
            is,
            should_be,
            step,
            ..
        } => remedy_out_of_step(among, should_be.saturating_sub(is), step),
        Finding::Escapes { .. } => "size the container to its contents, or let the contents \
             scroll inside it. A box that overflows is painted over whatever is next to it"
            .to_owned(),
        Finding::TextCut { .. } => "give the box the room the text needs, let it wrap, or draw \
             an ellipsis so that a reader can see something was held back"
            .to_owned(),
        Finding::UndeclaredPaint { ref nearest, .. } => match nearest.as_ref() {
            Some(near) => format!("write `{}` here", near.token),
            None => "declare this colour as a word beside the others, or take one of the words \
                 already declared"
                .to_owned(),
        },
        Finding::UndeclaredSize { ref nearest, .. } => match nearest.as_ref() {
            Some(near) => format!("write `{}` here", near.token),
            None => "set this text from a declared step; a size nothing names moves on its own \
                 when the scale moves"
                .to_owned(),
        },
        Finding::Echoed { .. } => "give the two one source, or say why they are two facts. Two \
             boxes reading two answers agree until the day one of them is written, and the page \
             has no way to notice"
            .to_owned(),
    }
}

fn remedy_out_of_step(among: Population, moved: i64, step: Option<&str>) -> String {
    let named = match step {
        Some(word) => format!(", which is one `{word}`"),
        None => String::new(),
    };
    let sentence = match among {
        Population::RegionsInTheMainColumn => {
            "let one authority set the inline margins - a region states its vertical rhythm, the \
             column states the spine"
        }
        Population::RowsOfANavigationColumn => {
            "draw the first mark of every row in one box of the same size - a dot centred in the \
             glyph's box lines up with the glyphs, a dot centred in its own does not"
        }
        Population::BoxesThatShareAHolder(_) => {
            "take the padding, the border or the width this box states from the same token its \
             neighbours take theirs from - a near miss is a number somebody typed twice"
        }
    };
    format!("move it {moved:+}{named}: {sentence}")
}

fn site(one: &Deviation<'_>, sources: &Sources) -> String {
    match sources.locate(&one.at.class) {
        Some(found) => format!("{found} \u{b7} {}", says(one)),

        None if one.at.class.is_empty() => says(one),
        None => format!("{} class=\"{}\"", says(one), one.at.class),
    }
}

fn among_says(among: Population) -> String {
    match among {
        Population::RegionsInTheMainColumn => {
            "a page has one left edge: every region in the main column starts at the same x"
                .to_owned()
        }
        Population::RowsOfANavigationColumn => {
            "every clickable row in a navigation column starts its first mark at the same x"
                .to_owned()
        }
        Population::BoxesThatShareAHolder(side) => format!(
            "boxes one container lays out share a {}, or sit far enough off it to be a different \
             column",
            side.of()
        ),
    }
}

fn among_axis(among: Population) -> &'static str {
    match among {
        Population::RegionsInTheMainColumn => "left edge",
        Population::RowsOfANavigationColumn => "first mark",
        Population::BoxesThatShareAHolder(side) => side.of(),
    }
}

fn pixels(px_x100: u32) -> String {
    let whole = px_x100.checked_div(100).unwrap_or(0);
    let rest = px_x100.checked_rem(100).unwrap_or(0);
    if rest == 0 {
        format!("{whole}px")
    } else {
        format!("{whole}.{rest:02}px")
    }
}

fn nearby(near: Option<&Near<'_>>) -> String {
    match near {
        Some(near) => format!("; nearest `{}`, {} off", near.token, near.apart),
        None => "; nothing declared is near it".to_owned(),
    }
}
