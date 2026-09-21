// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The role layer: what a surface is *for*, judged against a closed
//! vocabulary (xtask-SPEC.md section 8-8).
//!
//! The rungs are the authority on values and the six assertions beside
//! this file judge them. They never answered the question a view asks -
//! *what fills a control that has lifted off the page* - so before this
//! gate existed every view answered it where it stood: two hundred and
//! thirty-six hand-typed `bg-g2` and `border-g3` spellings, no two of
//! which could be compared and none of which was the authority.
//!
//! **A role is a single hop and nothing else.** `--color-raised:
//! var(--color-g2)`. Not an `oklch()` beside the rung it copies, which
//! would be that value's second home and would have to be re-tuned by
//! hand whenever the rung moved; and not a hex literal, which the six
//! assertions cannot read at all - `tables.rs` keeps only declarations
//! whose value parses as `oklch()`, so a `--color-raised: #2a2a2a`
//! would have been *silently ignored*, not refused. That silence is
//! what this file closes.
//!
//! **The hop is also what makes one declaration serve both lightings.**
//! The light block restates every rung; a role that points at a rung
//! follows it without being mentioned there, and a role that carried
//! its own value would need a second declaration that could disagree.
//!
//! Four rules, and together they make the vocabulary closed in both
//! directions: a name this file does not know cannot be declared, a
//! name it knows cannot go undeclared, a role nothing spells is a name
//! with no reader, and a rung spelled outside the stylesheet is a view
//! answering the question again on its own.

use std::path::Path;

use super::THEME;
use crate::report::{Violation, XtaskError};
use crate::walk;

/// Where the views that must speak in roles live. `walk` states the
/// directory; this gate only says that it is the one it reads.
use crate::walk::CLIENT_SRC as VIEWS;

/// Which file kinds carry a class name. Wider than what the client is
/// written in, because `index.html` carries classes too and a rung
/// spelled there draws exactly as wrongly as one spelled in a view.
const VIEW_EXTS: [&str; 3] = ["tsx", "ts", "html"];

/// The closed vocabulary.
///
/// Ordered as the stylesheet declares them - surfaces outward from the
/// page, then the marks, then the edges, then the one ink - because a
/// reader looking for "which one do I want" reads this list and not the
/// cascade.
///
/// **Adding a row is a design decision and costs one.** The draft of
/// this layer carried `inert` and `resting` as two marks one rung
/// apart, and no reader could have said which a given dot was; they are
/// one row now. A role earns its place only when somebody can state the
/// question it answers in a sentence that does not mention a rung.
const ROLES: [&str; 21] = [
    // The surfaces, outward from the page.
    "page",
    "chrome",
    "raised",
    "raised-hover",
    // Three fills that are not a surface a person navigates.
    "speech",
    "track",
    "disabled",
    // The one fill of a mark that reports rather than offers.
    "mark",
    // The edges, by what can be done to the box they bound.
    "edge",
    "edge-panel",
    "edge-input",
    // The one ink that goes on top of a filled coloured token.
    "on-accent",
    // The city drawing, which encodes depth in a picture rather than
    // what can be done to a box, and therefore has its own prefix.
    "drawn-hollow",
    "drawn-solid",
    "drawn-solid-lit",
    "drawn-line",
    "drawn-edge",
    "drawn-part",
    "drawn-stem",
    "drawn-aside",
    "drawn-figure",
];

/// A `--color-*` declaration that is neither a rung nor a text token:
/// its name as the stylesheet spells it, and its value.
struct Named<'a> {
    name: &'a str,
    value: &'a str,
}

pub(super) fn judge_roles(root: &Path, source: &str) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let rungs: Vec<&str> = declared(source)
        .filter(|held| is_rung(held.name))
        .map(|held| held.name)
        .collect();
    let roles: Vec<Named<'_>> = declared(source)
        .filter(|held| !is_rung(held.name) && !is_text(held.name) && !held.value.starts_with("oklch("))
        .collect();

    // 1. Everything that is not a rung, a text token or a coloured token
    //    is a role: a known name, and a single hop to a declared rung.
    for held in &roles {
        if !ROLES.contains(&held.name) {
            violations.push(refuse(
                "every surface role is one of the names `xtask::color::roles` knows",
                format!(
                    "--color-{} is declared and is not in the closed vocabulary",
                    held.name
                ),
                "add the role to ROLES with the question it answers, or spell an existing one",
            ));
            continue;
        }
        match hop(held.value) {
            Some(rung) if rungs.contains(&rung) => {}
            Some(rung) => violations.push(refuse(
                "every surface role hops to a rung the ramp declares",
                format!("--color-{} hops to --color-{rung}, which is not a rung", held.name),
                "point the role at one of --color-g0 … --color-g10",
            )),
            None => violations.push(refuse(
                "every surface role is a single hop to a rung, never a value of its own",
                format!("--color-{} is `{}`", held.name, held.value),
                "write `var(--color-gN)`; a literal beside the rung it copies is that value's second home",
            )),
        }
    }

    // 2. A name this file knows and the stylesheet never declares is a
    //    role every view would spell into nothing.
    for role in ROLES {
        let found = roles.iter().filter(|held| held.name == role).count();
        if found != 1 {
            violations.push(refuse(
                "every role in the closed vocabulary is declared exactly once",
                format!("--color-{role} is declared {found} times"),
                "declare it in the role block of the theme, or drop the row from ROLES",
            ));
        }
    }

    // 3. A role nothing spells is a name with no reader.
    let spelled = spellings(root)?;
    for role in ROLES {
        if !spelled.iter().any(|text| names(text, role)) {
            violations.push(refuse(
                "every role has at least one reader",
                format!("--color-{role} is declared and nothing under {VIEWS} spells it"),
                "use the role, or delete it: a surface nobody fills does not need a name",
            ));
        }
    }

    // 4. A rung spelled outside the stylesheet is a view deciding for
    //    itself how bright a surface should be - which is the defect.
    violations.extend(rungs_outside_the_theme(root)?);
    Ok(violations)
}

/// Every `--color-<name>: <value>` the stylesheet declares.
fn declared(source: &str) -> impl Iterator<Item = Named<'_>> {
    source.lines().filter_map(|line| {
        let (name, tail) = line.trim().strip_prefix("--color-")?.split_once(':')?;
        let value = tail.split(';').next()?.trim();
        let name = name.trim();
        (!value.is_empty() && !name.contains('*')).then_some(Named { name, value })
    })
}

/// `var(--color-g2)` is a hop to `g2`; anything else is not a hop.
fn hop(value: &str) -> Option<&str> {
    value
        .strip_prefix("var(--color-")?
        .strip_suffix(')')
        .filter(|inner| !inner.contains(['(', ')', ',', ' ']))
}

fn is_rung(name: &str) -> bool {
    name.strip_prefix('g')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

fn is_text(name: &str) -> bool {
    name == "text" || name.starts_with("text-")
}

/// The text of every view file, read once for both the reader check and
/// the rung scan.
fn spellings(root: &Path) -> Result<Vec<String>, XtaskError> {
    let mut held = Vec::new();
    for path in walk::files_with_ext(&root.join(VIEWS), &VIEW_EXTS)? {
        if walk::rel(root, &path) == THEME {
            continue;
        }
        held.push(walk::read_text(&path)?);
    }
    Ok(held)
}

/// Whether a file spells a role as a whole word after a utility prefix -
/// `bg-raised` names `raised`, and `bg-raised-hover` does not.
fn names(text: &str, role: &str) -> bool {
    text.match_indices(role).any(|(at, _)| {
        let before = text.get(..at).and_then(|head| head.bytes().next_back());
        let after = text
            .get(at.saturating_add(role.len())..)
            .and_then(|tail| tail.bytes().next());
        before == Some(b'-')
            && !after.is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

/// Every place a view names a rung directly.
fn rungs_outside_the_theme(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for path in walk::files_with_ext(&root.join(VIEWS), &VIEW_EXTS)? {
        let rel = walk::rel(root, &path);
        if rel == THEME {
            continue;
        }
        let text = walk::read_text(&path)?;
        for (index, line) in text.lines().enumerate() {
            if let Some(spelling) = rung_utility(line) {
                let at = index.saturating_add(1);
                violations.push(Violation {
                    gate: "color",
                    location: format!("{rel}:{at}"),
                    rule: "a view spells the role a surface has, never the rung it resolves to"
                        .to_owned(),
                    violation: format!("`{spelling}` names a rung of the ramp"),
                    alternative: format!(
                        "spell the role whose job this is; the vocabulary is the ROLES table in \
                         xtask/src/color/roles.rs, and {THEME} maps each one to a rung"
                    ),
                });
            }
        }
    }
    Ok(violations)
}

/// Whether a line carries `bg-g2`, `border-g3`, `text-g0` or any other
/// utility built straight on a rung.
///
/// The prefix has to be one of the three that take a colour: `gap-`,
/// `grid-` and `group/` all start with a `g` that is not a rung, and a
/// bare search for `-g` would report every one of them.
fn rung_utility(line: &str) -> Option<String> {
    for prefix in ["bg-", "border-", "text-", "fill-", "stroke-", "ring-", "outline-"] {
        let mut rest = line;
        while let Some(at) = rest.find(prefix) {
            let tail = rest.get(at.saturating_add(prefix.len())..)?;
            let name: String = tail
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric())
                .map(char::from)
                .collect();
            let ends = tail
                .as_bytes()
                .get(name.len())
                .copied()
                .is_none_or(|byte| byte != b'-' && byte != b'_');
            if ends && is_rung(&name) {
                return Some(format!("{prefix}{name}"));
            }
            rest = rest.get(at.saturating_add(prefix.len())..)?;
        }
    }
    None
}

fn refuse(rule: &str, violation: String, alternative: &str) -> Violation {
    Violation {
        gate: "color",
        location: THEME.to_owned(),
        rule: rule.to_owned(),
        violation,
        alternative: alternative.to_owned(),
    }
}
