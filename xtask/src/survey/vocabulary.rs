// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the page declared, against what it painted.
//!
//! **This is the reading that only rendering can take.** A class is
//! conditional, so counting the uses of a token in source tells you
//! which tokens somebody typed, not which ones a person was shown. A
//! palette of eleven rungs where six are ever painted is eleven homes
//! for one decision with five of them empty, and the source says
//! nothing about it.
//!
//! **The declared words are read off the page, not kept here.** The
//! engine resolves every custom property on the root element the way
//! it would paint it, so this module holds no table of its own: a
//! stylesheet that renames a step renames it here in the same run. That
//! is also why the reading survives a change of stack - a theme
//! declared as constants rather than as custom properties is the same
//! name-to-value table read through a different collector.

use std::collections::BTreeMap;

use super::{Deviation, Finding, Near, Page, Paint, PaintSource, Surface};

/// The vocabulary a page declares about itself, as the engine resolved
/// it.
pub(crate) struct Declared {
    colours: Vec<(String, Paint)>,
    /// The type steps, in hundredths of a pixel.
    type_steps: Vec<(String, u32)>,
    /// The spacing steps, in hundredths of a pixel.
    spacing: Vec<(String, u32)>,
}

/// How far a painted colour may sit from a declared one and still be
/// reported as that one's near miss rather than as nothing in
/// particular.
///
/// Wider than a rounding error and narrower than the gap between two
/// rungs of this client's ramp, which is about thirteen levels at its
/// tightest.
const SHADE: u16 = 8;

impl Declared {
    pub(crate) fn of(
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

    /// Whether the probe read no vocabulary at all, which is an engine
    /// that does not enumerate custom properties rather than a page
    /// with no palette. The two have to be told apart, because judging
    /// against an empty vocabulary reports every colour on the page.
    pub(crate) fn is_empty(&self) -> bool {
        self.colours.is_empty()
    }

    /// Whether the page declares a word for this colour.
    fn names(&self, painted: Paint) -> bool {
        self.colours.iter().any(|(_, value)| *value == painted)
    }

    /// The declared word nearest a colour the page never declared.
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

    /// Whether the page declares a step at this size.
    fn steps(&self, px_x100: u32) -> bool {
        self.type_steps.iter().any(|(_, value)| *value == px_x100)
    }

    /// The declared step nearest a size the page never declared, when
    /// one is near enough that somebody meant it.
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

    /// The spacing step a whole number of pixels is, when it is one.
    ///
    /// This is what lets a correction be written as the word a person
    /// would type rather than as a number: a box three pixels off its
    /// column is three pixels, but a box eight pixels off it is one
    /// `snug` somebody applied twice.
    pub(crate) fn spacing_named(&self, px: i64) -> Option<&str> {
        let hundredths = u32::try_from(px.saturating_mul(100)).ok()?;
        self.spacing
            .iter()
            .find(|(_, value)| *value == hundredths)
            .map(|(token, _)| token.as_str())
    }

    /// Every colour the page declares, in the order it declared them.
    fn words(&self) -> impl Iterator<Item = &str> {
        self.colours.iter().map(|(token, _)| token.as_str())
    }
}

/// Every colour the page painted is a word the page declared.
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

/// Every size the page set is a step the page declared.
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

/// The words the page declared and never painted, and how the painting
/// it did do was distributed.
///
/// A palette is a set of decisions, and a decision nobody was ever
/// shown is a decision with no consequence: it survives every review
/// because reading the stylesheet cannot tell it from one that carries
/// half the page.
pub(crate) fn unpainted(page: &Page) -> Option<String> {
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
