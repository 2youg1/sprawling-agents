// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Glass: whether text on a see-through surface of the edge layer is
//! still legible (tools/xtask/Spec.lean §8-51, client/Spec.lean §4-43).
//!
//! **The opacity judged is the opacity drawn.** `--glass-opacity` is the
//! number `theme.css` mixes the glass role with, and it is the number
//! read here; a second declaration kept for this gate would be a floor
//! that could drift away from what the page draws.
//!
//! **The backdrop is the brightest surface, not the brightest ink.** The
//! glass blurs what is behind it by 24 px, which spreads the words there
//! into the surface they sit on; what is left is a surface, and the
//! brightest one a page fills an area with is a raised control under the
//! pointer. Judged against ink, the opacity would have to reach 97 per
//! cent and the glass would be a solid.

use super::contrast::apca_lc_over;
use super::roles::rung_of;
use super::tables::{grey_ramp, parse_text_tokens};
use super::{Mode, THEME};
use crate::report::Violation;

/// The role the glass is filled with, and the surface it is judged over.
const GLASS: &str = "glass";
const BACKDROP: &str = "raised-hover";
/// The text token that has to stay legible on the glass, as
/// `parse_text_tokens` names it.
const TEXT: &str = "TEXT";
/// The declaration that says how much of the role covers the backdrop.
const OPACITY: &str = "--glass-opacity:";

/// Text on the glass over the backdrop, in one lighting.
///
/// `written` is the stylesheet as written, which holds the roles once
/// for both lightings; `read` is that lighting's reading of it, which
/// holds the rungs' values.
pub(super) fn judge_glass(written: &str, read: &str, mode: Mode) -> Vec<Violation> {
    let Some(percent) = opacity(written) else {
        return vec![refuse(
            format!(
                "{}: `--glass-opacity` is not declared as a whole percent from 1 to 100",
                mode.name()
            ),
            "declare `--glass-opacity: <n>%;` in the theme's root block",
        )];
    };
    let rungs = grey_ramp(read);
    let lightness = |role: &str| {
        let rung = rung_of(written, role)?.to_uppercase();
        rungs
            .iter()
            .find(|(name, _)| *name == rung)
            .map(|(_, l)| *l)
    };
    let text = parse_text_tokens(read)
        .into_iter()
        .find(|(name, _, _)| name == TEXT);
    let (Some(fill), Some(backdrop), Some((_, ink, tier))) =
        (lightness(GLASS), lightness(BACKDROP), text)
    else {
        return vec![refuse(
            format!(
                "{}: the glass role, the backdrop role or the text token cannot be read",
                mode.name()
            ),
            "keep `--color-glass` and `--color-raised-hover` as single hops to a rung, and `--color-text` with its `--tier-text`",
        )];
    };
    let reached = apca_lc_over(ink, fill, backdrop, percent);
    if reached + 0.05 >= f64::from(tier) {
        return Vec::new();
    }
    vec![refuse(
        format!(
            "{}: text on glass at {percent}% over `{BACKDROP}` reaches Lc {reached:.1} and claims Lc {tier}",
            mode.name()
        ),
        "raise `--glass-opacity`, or move `--color-glass` to a rung text reaches its tier on",
    )]
}

/// `--glass-opacity: 75%;` as 75, when it is a whole percent in range.
fn opacity(written: &str) -> Option<u16> {
    written.lines().find_map(|line| {
        let value = line
            .trim()
            .strip_prefix(OPACITY)?
            .trim()
            .strip_suffix(';')?
            .trim();
        value
            .strip_suffix('%')?
            .parse::<u16>()
            .ok()
            .filter(|percent| (1..=100).contains(percent))
    })
}

fn refuse(violation: String, alternative: &str) -> Violation {
    Violation {
        gate: "color",
        location: THEME.to_owned(),
        rule: "text on glass reaches its tier over the brightest surface behind it".to_owned(),
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
    use super::*;

    /// The glass lines of the shipped stylesheet, around the rungs and the
    /// text token both lightings read.
    fn sheet(percent: &str) -> String {
        format!(
            "
  --color-g0: oklch(0.145 0.014 250);
  --color-g1: oklch(0.172 0.014 250);
  --color-g2: oklch(0.215 0.014 250);
  --color-g3: oklch(0.265 0.014 250);
  --color-raised-hover: var(--color-g3);
  --color-glass: var(--color-g2);
  --color-text: oklch(0.930 0.014 250);
  --tier-text: 90;
  {percent}
:root[data-theme=\"light\"] {{
  --color-g0: oklch(0.978 0.014 250);
  --color-g1: oklch(0.962 0.014 250);
  --color-g2: oklch(0.940 0.014 250);
  --color-g3: oklch(0.905 0.014 250);
  --color-text: oklch(0.230 0.014 250);
}}
"
        )
    }

    fn judged(written: &str) -> Vec<String> {
        Mode::ALL
            .into_iter()
            .flat_map(|mode| judge_glass(written, &super::super::reading(written, mode), mode))
            .map(|found| found.violation)
            .collect()
    }

    #[test]
    fn the_shipped_opacity_keeps_text_legible_in_both_lightings() {
        assert_eq!(
            judged(&sheet("--glass-opacity: 75%;")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn an_opacity_too_low_for_the_light_page_is_refused_there_alone() {
        let found = judged(&sheet("--glass-opacity: 40%;"));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found
                .iter()
                .all(|said| said.starts_with("the light page: text on glass at 40%")),
            "{found:?}"
        );
    }

    #[test]
    fn an_opacity_nobody_declared_is_refused() {
        assert_eq!(judged(&sheet("")).len(), 2);
        assert_eq!(judged(&sheet("--glass-opacity: 0.75;")).len(), 2);
    }
}
