// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use super::{Deviation, Finding, Near, Page, Paint, PaintSource, Surface};

pub struct Declared {
    colours: Vec<(String, Paint)>,

    type_steps: Vec<(String, u32)>,

    spacing: Vec<(String, u32)>,
}

const SHADE: u16 = 8;

impl Declared {
    pub fn of(
        colours: Vec<(String, Paint)>,
        type_steps: Vec<(String, u32)>,
        spacing: Vec<(String, u32)>,
    ) -> Declared {
        Declared {
            colours,
            type_steps,
            spacing,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.colours.is_empty()
    }

    fn names(&self, painted: Paint) -> bool {
        self.colours.iter().any(|(_, value)| *value == painted)
    }

    fn nearest(&self, painted: Paint) -> Option<Near<'_>> {
        self.colours
            .iter()
            .map(|(token, value)| Near {
                token,
                apart: value.apart(painted),
            })
            .filter(|near| near.apart <= SHADE)
            .min_by_key(|near| near.apart)
    }

    fn steps(&self, px_x100: u32) -> bool {
        self.type_steps.iter().any(|(_, value)| *value == px_x100)
    }

    fn nearest_step(&self, px_x100: u32) -> Option<Near<'_>> {
        self.type_steps
            .iter()
            .filter_map(|(token, value)| {
                u16::try_from(value.abs_diff(px_x100))
                    .ok()
                    .map(|apart| Near { token, apart })
            })
            .min_by_key(|near| near.apart)
    }

    pub fn spacing_named(&self, px: i64) -> Option<&str> {
        let hundredths = u32::try_from(px.saturating_mul(100)).ok()?;
        self.spacing
            .iter()
            .find(|(_, value)| *value == hundredths)
            .map(|(token, _)| token.as_str())
    }

    fn words(&self) -> impl Iterator<Item = &str> {
        self.colours.iter().map(|(token, _)| token.as_str())
    }
}

pub(super) fn every_colour_is_a_declared_word<'a>(page: &'a Page, out: &mut Vec<Deviation<'a>>) {
    if page.painted_by == PaintSource::TheSystem {
        return;
    }
    for held in page.drawn.iter().filter(|held| held.shows()) {
        let mut readings = Vec::new();
        if let Some(fill) = held.fill {
            readings.push((Surface::Fill, fill));
        }
        if let Some(ink) = held.text.as_ref().and_then(|text| text.ink) {
            readings.push((Surface::Ink, ink));
        }
        for (on, painted) in readings {
            if page.declared.names(painted) {
                continue;
            }
            out.push(Deviation {
                at: held,
                finding: Finding::UndeclaredPaint {
                    painted,
                    on,
                    nearest: page.declared.nearest(painted),
                },
            });
        }
    }
}

pub(super) fn every_size_is_a_declared_step<'a>(page: &'a Page, out: &mut Vec<Deviation<'a>>) {
    for held in page.drawn.iter().filter(|held| held.shows()) {
        let Some(text) = held.text.as_ref() else {
            continue;
        };
        if page.declared.steps(text.px_x100) {
            continue;
        }
        out.push(Deviation {
            at: held,
            finding: Finding::UndeclaredSize {
                px_x100: text.px_x100,
                nearest: page.declared.nearest_step(text.px_x100),
            },
        });
    }
}

pub fn unpainted(page: &Page) -> Option<String> {
    if page.painted_by == PaintSource::TheSystem {
        return None;
    }
    let mut uses: BTreeMap<&str, usize> = page.declared.words().map(|word| (word, 0)).collect();
    if uses.is_empty() {
        return None;
    }
    for held in page.drawn.iter().filter(|held| held.shows()) {
        let painted = held
            .fill
            .into_iter()
            .chain(held.text.as_ref().and_then(|text| text.ink));
        for colour in painted {
            for (token, value) in &page.declared.colours {
                if *value == colour
                    && let Some(count) = uses.get_mut(token.as_str())
                {
                    *count = count.saturating_add(1);
                }
            }
        }
    }
    let idle: Vec<&str> = uses
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(token, _)| *token)
        .collect();
    if idle.is_empty() {
        return None;
    }
    Some(format!(
        "{} of {} declared colours are never painted here: {}",
        idle.len(),
        uses.len(),
        idle.join(" ")
    ))
}
