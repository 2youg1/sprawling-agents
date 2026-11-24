// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::sheet::{self, Reading};
use crate::theme;
use crate::walk;

const SCAN_EXTS: [&str; 4] = ["ts", "tsx", "css", "html"];

const NUMERIC_FUNCTIONS: [&str; 2] = ["linear(", "steps("];

const MACHINE_VARIANTS: [&str; 2] = ["motion-reduce:", "motion-safe:"];

const MOTION_TOKENS: [&str; 4] = [
    "--ease-",
    "--transition-duration-",
    "--animate-",
    "--default-transition-",
];

const TIMED: [&str; 2] = ["transition", "animation"];

#[derive(Clone, Copy)]
enum Refused {
    Literal(&'static str),

    MachineVariant(&'static str),

    Duration,

    Keyframes,

    Token(&'static str),
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for path in walk::client_files(root, &SCAN_EXTS)? {
        let rel = walk::rel(root, &path);
        if theme::is_theme(&rel) {
            continue;
        }
        let text = walk::read_text(&path)?;
        let mut timing = false;
        for line in sheet::lines(&rel, &text) {
            let in_sheet = match line.reading {
                Reading::Sheet => sheet_refused_at(line.text, &mut timing),
                Reading::Code => None,
            };
            if let Some(refused) = literal_at(line.text)
                .map(Refused::Literal)
                .or_else(|| machine_variant(line.text).map(Refused::MachineVariant))
                .or(in_sheet)
            {
                violations.push(refused.at(&rel, line.number));
            }
        }
    }
    Ok(violations)
}

impl Refused {
    fn at(self, rel: &str, number: usize) -> Violation {
        let tokens = "spell `duration-short`, `duration-panel` or `duration-page` and \
                      `ease-arrive` or `ease-leave` (docs/frontend-method.md §4-43), in a \
                      stylesheet through `var(--transition-duration-*)` and `var(--ease-*)`; a \
                      fourth duration or a third curve is declared in the theme's motion tokens \
                      first";
        let named_once = "a transition's curve and duration are named once, in the client's theme";
        let (rule, violation, alternative) = match self {
            Self::Literal(spelling) => (
                named_once,
                format!(
                    "`{spelling}` outside the theme ({} and the parts it imports)",
                    theme::ENTRY
                ),
                tokens.to_owned(),
            ),
            Self::Duration => (
                named_once,
                "a transition or an animation spells its duration in seconds outside the theme"
                    .to_owned(),
                tokens.to_owned(),
            ),
            Self::Token(prefix) => (
                named_once,
                format!("a `{prefix}*` token is declared outside the theme"),
                tokens.to_owned(),
            ),
            Self::Keyframes => (
                "an animation is named once, in the client's theme",
                "`@keyframes` outside the theme".to_owned(),
                "declare the keyframes and a class that runs them on the duration tokens in the \
                 theme's motion part, and write that class here"
                    .to_owned(),
            ),
            Self::MachineVariant(variant) => (
                "motion stops for the person's choice as well as the machine's",
                format!("`{variant}` hears only the operating system's setting"),
                "write `still:`, the theme's variant that also holds where the person turned \
                 motion off"
                    .to_owned(),
            ),
        };
        Violation {
            gate: "motion",
            location: format!("{rel}:{number}"),
            rule: rule.to_owned(),
            violation,
            alternative,
        }
    }
}

fn machine_variant(line: &str) -> Option<&'static str> {
    MACHINE_VARIANTS.into_iter().find(|variant| {
        line.match_indices(variant).any(|(at, _)| {
            line.get(..at)
                .and_then(|head| head.bytes().next_back())
                .is_none_or(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')))
        })
    })
}

fn sheet_refused_at(line: &str, timing: &mut bool) -> Option<Refused> {
    let timed = timed_literal(line, timing);
    if line.contains("@keyframes") {
        return Some(Refused::Keyframes);
    }
    MOTION_TOKENS
        .into_iter()
        .find(|prefix| sheet::declared(line, prefix).next().is_some())
        .map(Refused::Token)
        .or(timed.then_some(Refused::Duration))
}

fn timed_literal(line: &str, timing: &mut bool) -> bool {
    let mut rest = line;
    let mut found = false;
    loop {
        if !*timing {
            let Some(after) = value_start(rest) else {
                return found;
            };
            *timing = true;
            rest = rest.get(after..).unwrap_or("");
        }
        let end = rest.find([';', '}']);
        let value = end.and_then(|at| rest.get(..at)).unwrap_or(rest);
        found = found
            || value
                .split(|c: char| c.is_whitespace() || matches!(c, ',' | '(' | ')'))
                .any(is_duration);
        let Some(at) = end else {
            return found;
        };
        *timing = false;
        rest = rest.get(at.saturating_add(1)..).unwrap_or("");
    }
}

fn value_start(rest: &str) -> Option<usize> {
    TIMED
        .into_iter()
        .flat_map(|word| rest.match_indices(word))
        .filter_map(|(at, word)| {
            let opens = rest
                .get(..at)
                .and_then(|head| head.bytes().next_back())
                .is_none_or(|byte| byte.is_ascii_whitespace() || matches!(byte, b'{' | b';'));
            let tail = rest.get(at.saturating_add(word.len())..)?;
            let colon = tail.strip_prefix("-duration").unwrap_or(tail).trim_start();
            (opens && colon.starts_with(':'))
                .then(|| rest.len().checked_sub(colon.len()))
                .flatten()
                .map(|at| at.saturating_add(1))
        })
        .min()
}

fn is_duration(token: &str) -> bool {
    token
        .strip_suffix("ms")
        .or_else(|| token.strip_suffix('s'))
        .is_some_and(|number| {
            number.bytes().any(|byte| byte.is_ascii_digit())
                && number
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'.')
        })
}

fn literal_at(line: &str) -> Option<&'static str> {
    if line.contains("cubic-bezier(") {
        return Some("cubic-bezier(");
    }
    if let Some(function) = NUMERIC_FUNCTIONS
        .into_iter()
        .find(|function| takes_a_number(line, function))
    {
        return Some(function);
    }
    tailwind_literal(line)
}

fn takes_a_number(line: &str, function: &str) -> bool {
    line.match_indices(function).any(|(at, _)| {
        line.get(at.saturating_add(function.len())..)
            .and_then(|rest| rest.trim_start().bytes().next())
            .is_some_and(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'-'))
    })
}

fn tailwind_literal(line: &str) -> Option<&'static str> {
    let starts = |at: usize| {
        line.get(..at)
            .and_then(|head| head.bytes().next_back())
            .is_none_or(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')))
    };
    for (utility, spelled) in [("duration-", "duration-<n>"), ("ease-[", "ease-[")] {
        for (at, _) in line.match_indices(utility) {
            let next = line
                .get(at.saturating_add(utility.len())..)
                .and_then(|rest| rest.bytes().next());
            let literal = utility == "ease-["
                || next.is_some_and(|byte| byte.is_ascii_digit() || byte == b'[');
            if literal && starts(at) {
                return Some(spelled);
            }
        }
    }
    None
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::{check, literal_at};

    #[test]
    fn a_part_the_theme_imports_may_spell_a_curve() {
        let root = std::env::temp_dir().join(format!("motion-theme-part-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let written = [
            (
                "RefRain/src/theme.css",
                "@import \"./theme/tokens-motion.css\";
",
            ),
            (
                "RefRain/src/theme/tokens-motion.css",
                "@theme {
  --ease-arrive: cubic-bezier(0, 0, 0.1, 1);
}
",
            ),
            (
                "RefRain/src/views/panel.tsx",
                "<div style=\"transition: x 1s cubic-bezier(0, 0, 0.1, 1)\"></div>
",
            ),
        ];
        for (rel, body) in written {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
        }

        let found = check(&root).unwrap();
        assert_eq!(
            found
                .iter()
                .map(|v| v.location.as_str())
                .collect::<Vec<_>>(),
            ["RefRain/src/views/panel.tsx:1"]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_three_timing_functions_are_refused_and_a_snippet_named_steps_is_not() {
        assert_eq!(
            literal_at("transition: x 1s cubic-bezier(0.2, 0, 0, 1);"),
            Some("cubic-bezier(")
        );
        assert_eq!(
            literal_at("animation-timing-function: linear(0, 0.5 50%, 1);"),
            Some("linear(")
        );
        assert_eq!(
            literal_at("animation: tick 1s steps(4) infinite;"),
            Some("steps(")
        );
        assert_eq!(literal_at("{#snippet steps(each: Walk)}"), None);
        assert_eq!(literal_at("const rate = linear(width);"), None);
    }

    #[test]
    fn tailwind_spells_a_duration_or_a_curve_as_a_literal_only_with_a_number_or_brackets() {
        assert_eq!(
            literal_at(r#"class="transition-colors duration-150""#),
            Some("duration-<n>")
        );
        assert_eq!(
            literal_at(r#"class="hover:duration-[90ms]""#),
            Some("duration-<n>")
        );
        assert_eq!(literal_at(r#"class="ease-[cubic]""#), Some("ease-["));
        assert_eq!(
            literal_at(r#"class="duration-panel ease-arrive open:ease-leave""#),
            None
        );
        assert_eq!(
            literal_at("transition: x var(--transition-duration-short);"),
            None
        );
        assert_eq!(literal_at("animation: none;"), None);
    }

    #[test]
    fn a_component_style_reads_the_tokens_and_hears_the_person() {
        let root = std::env::temp_dir().join(format!("motion-sheet-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let written = [
            (
                "RefRain/src/views/panel.tsx",
                "<div class=\"transition-opacity motion-reduce:transition-none\"></div>
<p class=\"still:transition-none\"></p>
",
            ),
            (
                "RefRain/src/views/panel.css",
                "  .a { transition: opacity 200ms; }
  .b { transition: opacity var(--transition-duration-short) var(--ease-arrive); }
  @keyframes x {}
  .c {
    animation: x var(--transition-duration-panel)
      1.5s both;
  }
  .d { transition-duration: .2s; transition-property: opacity; }
",
            ),
            (
                "RefRain/src/views/plain.css",
                ".e { --ease-mine: var(--ease-arrive); }
.f { animation-delay: 40ms; }
",
            ),
            (
                "RefRain/swap/views/parts/x.look.solid.tsx",
                "<div class=\"motion-safe:duration-short\"></div>
",
            ),
        ];
        for (rel, body) in written {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
        }

        let found = check(&root).unwrap();
        assert_eq!(
            found
                .iter()
                .map(|v| v.location.as_str())
                .collect::<Vec<_>>(),
            [
                "RefRain/src/views/panel.css:1",
                "RefRain/src/views/panel.css:3",
                "RefRain/src/views/panel.css:6",
                "RefRain/src/views/panel.css:8",
                "RefRain/src/views/panel.tsx:1",
                "RefRain/src/views/plain.css:1",
                "RefRain/swap/views/parts/x.look.solid.tsx:1",
            ]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
