// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::Mode;
use super::contrast::apca_lc_over;
use super::roles::rung_of;
use super::tables::{grey_ramp, parse_text_tokens, tier_slack};
use crate::report::Violation;
use crate::theme;

const GLASS: &str = "glass";
const BACKDROP: &str = "raised-hover";

const TEXT: &str = "TEXT";

pub(super) const OPACITY: &str = "--glass-opacity:";

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
    let (Some(fill), Some(backdrop), Some((_, ink, tier)), Some(slack)) = (
        lightness(GLASS),
        lightness(BACKDROP),
        text,
        tier_slack(read),
    ) else {
        return vec![refuse(
            format!(
                "{}: the glass role, the backdrop role, the text token or the tier slack cannot be read",
                mode.name()
            ),
            "keep `--color-glass` and `--color-raised-hover` as single hops to a rung, `--color-text` with its `--tier-text`, and `--tier-slack`",
        )];
    };
    let reached = apca_lc_over(ink, fill, backdrop, percent);
    if reached + slack >= f64::from(tier) {
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
        location: theme::ENTRY.to_owned(),
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
  --tier-slack: 0.05;
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
