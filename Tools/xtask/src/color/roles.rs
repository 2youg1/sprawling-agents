// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::sheet::{self, Reading};
use crate::theme;
use crate::walk;

use crate::walk::CLIENT_SRC as VIEWS;

const VIEW_EXTS: [&str; 4] = ["svelte", "ts", "html", "css"];

const ROLES: [&str; 22] = [

    "page",
    "chrome",
    "raised",
    "raised-hover",

    "glass",

    "speech",
    "track",
    "disabled",

    "mark",

    "edge",
    "edge-panel",
    "edge-input",

    "on-accent",

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
        .filter(|held| {
            !is_rung(held.name) && !is_text(held.name) && !held.value.starts_with("oklch(")
        })
        .collect();

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

    let spelled = spellings(root)?;
    for role in ROLES {
        let utility = source.contains(&format!("@utility {role} {{"));
        if !spelled
            .iter()
            .any(|text| names(text, role) || (utility && class_word(text, role)))
        {
            violations.push(refuse(
                "every role has at least one reader",
                format!("--color-{role} is declared and nothing under {VIEWS} spells it"),
                "use the role, or delete it: a surface nobody fills does not need a name",
            ));
        }
    }

    violations.extend(views_outside_the_theme(root, source)?);
    Ok(violations)
}

fn declared(source: &str) -> impl Iterator<Item = Named<'_>> {
    source.lines().filter_map(|line| {
        let (name, tail) = line.trim().strip_prefix("--color-")?.split_once(':')?;
        let value = tail.split(';').next()?.trim();
        let name = name.trim();
        (!value.is_empty() && !name.contains('*')).then_some(Named { name, value })
    })
}

pub(super) fn rung_of<'a>(source: &'a str, role: &str) -> Option<&'a str> {
    declared(source)
        .find(|held| held.name == role)
        .and_then(|held| hop(held.value))
}

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

fn spellings(root: &Path) -> Result<Vec<String>, XtaskError> {
    let mut held = Vec::new();
    for path in walk::files_with_ext(&root.join(VIEWS), &VIEW_EXTS)? {
        if theme::is_theme(&walk::rel(root, &path)) {
            continue;
        }
        held.push(walk::read_text(&path)?);
    }
    Ok(held)
}

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

fn class_word(text: &str, word: &str) -> bool {
    let continues = |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':');
    text.match_indices(word).any(|(at, _)| {
        let before = text.get(..at).and_then(|head| head.bytes().next_back());
        let after = text
            .get(at.saturating_add(word.len())..)
            .and_then(|tail| tail.bytes().next());
        !before.is_some_and(continues) && !after.is_some_and(continues)
    })
}

fn views_outside_the_theme(root: &Path, source: &str) -> Result<Vec<Violation>, XtaskError> {
    let known: Vec<&str> = declared(source).map(|held| held.name).collect();
    let mut violations = Vec::new();
    for path in walk::client_files(root, &VIEW_EXTS)? {
        let rel = walk::rel(root, &path);
        if theme::is_theme(&rel) {
            continue;
        }
        let text = walk::read_text(&path)?;
        for line in sheet::lines(&rel, &text) {
            let at = format!("{rel}:{}", line.number);
            let rung = rung_utility(line.text).or_else(|| {
                reads(line.text)
                    .find(|name| is_rung(name))
                    .map(|name| format!("var(--color-{name})"))
            });
            if let Some(spelling) = rung {
                violations.push(Violation {
                    gate: "color",
                    location: at.clone(),
                    rule: "a view spells the role a surface has, never the rung it resolves to"
                        .to_owned(),
                    violation: format!("`{spelling}` names a rung of the ramp"),
                    alternative: format!(
                        "spell the role whose job this is; the vocabulary is the ROLES table in \
                         Tools/xtask/src/color/roles.rs, and the theme ({}) maps each one to \
                         a rung",
                        theme::ENTRY
                    ),
                });
            }
            if let Some(name) =
                reads(line.text).find(|name| !is_rung(name) && !known.contains(name))
            {
                violations.push(Violation {
                    gate: "color",
                    location: at.clone(),
                    rule: "a view reads only the colours the theme declares".to_owned(),
                    violation: format!(
                        "`var(--color-{name})` reads a colour the theme never declares, so the \
                         engine draws nothing in its place and says nothing"
                    ),
                    alternative: "spell a role from the ROLES table in \
                                  Tools/xtask/src/color/roles.rs or a token the theme declares"
                        .to_owned(),
                });
            }
            let own = (line.reading == Reading::Sheet)
                .then(|| sheet::declared(line.text, "--color-").next())
                .flatten();
            if let Some(name) = own {
                violations.push(Violation {
                    gate: "color",
                    location: at,
                    rule: "colour is named once per client, in that client's theme file".to_owned(),
                    violation: format!("--color-{name} is declared outside the theme"),
                    alternative: format!(
                        "declare the colour in the theme ({} and the parts it imports) and read \
                         it here through var()",
                        theme::ENTRY
                    ),
                });
            }
        }
    }
    Ok(violations)
}

fn reads(line: &str) -> impl Iterator<Item = &str> {
    const OPEN: &str = "var(--color-";
    line.match_indices(OPEN).filter_map(|(at, _)| {
        let tail = line.get(at.saturating_add(OPEN.len())..)?;
        let length = tail
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
            .count();
        let name = tail.get(..length)?;
        let closes = tail.get(length..)?.trim_start().starts_with([')', ',']);
        (closes && !name.is_empty()).then_some(name)
    })
}

fn rung_utility(line: &str) -> Option<String> {
    for prefix in [
        "bg-", "border-", "text-", "fill-", "stroke-", "ring-", "outline-",
    ] {
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
        location: theme::ENTRY.to_owned(),
        rule: rule.to_owned(),
        violation,
        alternative: alternative.to_owned(),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::judge_roles;
    use crate::color::scan::tests::SVELTE_FIXTURE;

    #[test]
    fn a_component_style_reads_declared_roles_and_declares_no_colour() {
        let root = std::env::temp_dir().join(format!("color-roles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let path = root.join("client/src/views/panel.svelte");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, SVELTE_FIXTURE).unwrap();
        let source = "  --color-g3: oklch(0.3 0.014 250);
  --color-accent: oklch(0.6 calc(0.151 * var(--chroma)) 250);
";

        let found = judge_roles(&root, source).unwrap();
        assert_eq!(
            found
                .iter()
                .filter(|v| v.location.starts_with("client/src/views/"))
                .map(|v| v.location.as_str())
                .collect::<Vec<_>>(),
            [
                "client/src/views/panel.svelte:8",
                "client/src/views/panel.svelte:9",
                "client/src/views/panel.svelte:10",
            ]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
