// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a resident stops being able to do when one passage of the city
//! document is taken away.
//!
//! **`docs/City.md` is compiled into the binary and read by every
//! resident before anything else, so each passage of it charges rent on
//! every prefix the city assembles.** This module is the instrument that
//! prices that rent: it cuts the document into passages, removes one at
//! a time, and reports which capabilities a resident loses with it.
//!
//! **It is not a gate and must not become one.** The measurement is a
//! function of a document written for models, so its verdict moves when
//! the document is edited; wired to a red light it would only teach the
//! next editor to edit the corpus. The run lives behind `#[ignore]` in
//! this module's own tests and answers when somebody asks it.
//!
//! **It does not call a model**, for the reason [`crate::Shape`]'s suite
//! does not: a suite owning a provider cannot run offline, cannot be
//! replayed, and measures the network as much as the model. The
//! measurable stand-in is a cue — the phrase the document uses to grant
//! one capability. The capability survives the removal of a passage when
//! its cue does.

use kernel::{AxCode, AxError, ByteLen};

pub(crate) mod capabilities;

/// One thing a resident must be able to do, and the phrase that grants
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Capability {
    pub(crate) name: &'static str,
    pub(crate) cue: &'static str,
}

/// One passage of the document: a paragraph together with the bullet
/// list that expands it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Passage {
    pub(crate) index: u32,
    pub(crate) opening: String,
    pub(crate) removed: ByteLen,
}

/// What removing one passage costs a resident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Cost {
    /// Nothing in the corpus is granted here.
    Untouched,
    /// What it grants is said somewhere else too.
    Restated { also_said: Vec<&'static str> },
    /// This is the only place these are said.
    Sole { lost: Vec<&'static str> },
}

/// One passage and its price.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Charge {
    pub(crate) passage: Passage,
    pub(crate) cost: Cost,
}

/// The document, cut into passages, ready to be priced.
#[derive(Debug, Clone)]
pub(crate) struct Ablation {
    passages: Vec<String>,
    corpus: &'static [Capability],
}

impl Ablation {
    /// Cuts `document` into passages and holds them against `corpus`.
    ///
    /// A passage is a paragraph together with the bullet list that
    /// expands it: every list in this document belongs to the sentence
    /// above it, and pricing the two apart would price the typography
    /// rather than the meaning.
    ///
    /// # Errors
    /// Refuses a corpus holding a cue the whole document never says.
    /// That cue has drifted away from the text, and scoring it anyway
    /// would measure how stale the corpus is rather than how much the
    /// document earns.
    pub(crate) fn new(document: &str, corpus: &'static [Capability]) -> Result<Ablation, AxError> {
        for capability in corpus {
            if !document.contains(capability.cue) {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "ablate the city document",
                    capability.name.to_owned(),
                )
                .with_recovery(
                    "the document no longer says this cue; correct the corpus, not the document",
                ));
            }
        }
        Ok(Ablation {
            passages: cut(document),
            corpus,
        })
    }

    /// What each passage costs, in the document's own order.
    ///
    /// **The heavier verdict wins.** A passage that is the only place
    /// one capability is said and a second place another is said is
    /// reported as [`Cost::Sole`]: the restatement it also carries is
    /// free, and naming it beside a real loss would flatter the passage.
    pub(crate) fn charges(&self) -> Vec<Charge> {
        self.passages
            .iter()
            .enumerate()
            .map(|(index, passage)| Charge {
                passage: Passage {
                    index: u32::try_from(index).unwrap_or(u32::MAX),
                    opening: opening(passage),
                    removed: ByteLen::new(u64::try_from(passage.len()).unwrap_or(u64::MAX)),
                },
                cost: self.without(index, passage),
            })
            .collect()
    }

    /// The same charges, most expensive first.
    ///
    /// Ranked by how many capabilities go with the passage, then by how
    /// few bytes it takes with it — of two equal losses the shorter
    /// passage is the one earning its length — and last by position, so
    /// that two runs of this rank the same way.
    pub(crate) fn costliest_first(&self) -> Vec<Charge> {
        let mut charges = self.charges();
        charges.sort_by_key(|charge| {
            let lost = match &charge.cost {
                Cost::Sole { lost } => lost.len(),
                Cost::Restated { .. } | Cost::Untouched => 0,
            };
            (
                std::cmp::Reverse(lost),
                charge.passage.removed.get(),
                charge.passage.index,
            )
        });
        charges
    }

    /// Prices one passage by reading the document that is left without
    /// it.
    fn without(&self, index: usize, passage: &str) -> Cost {
        let remaining: String = self
            .passages
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != index)
            .map(|(_, text)| text.as_str())
            .collect::<Vec<&str>>()
            .join("\n\n");
        let mut lost = Vec::new();
        let mut also_said = Vec::new();
        for capability in self.corpus {
            if !passage.contains(capability.cue) {
                continue;
            }
            if remaining.contains(capability.cue) {
                also_said.push(capability.name);
            } else {
                lost.push(capability.name);
            }
        }
        if !lost.is_empty() {
            return Cost::Sole { lost };
        }
        if !also_said.is_empty() {
            return Cost::Restated { also_said };
        }
        Cost::Untouched
    }
}

/// Cuts a document into passages on blank lines, keeping a bullet list
/// with the paragraph it expands.
fn cut(document: &str) -> Vec<String> {
    let mut passages: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut after_blank = false;
    for line in document.lines() {
        if line.trim().is_empty() {
            after_blank = true;
            continue;
        }
        let continues = line.starts_with("- ") || current.is_empty();
        if after_blank && !continues {
            passages.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
        after_blank = false;
    }
    if !current.is_empty() {
        passages.push(current);
    }
    passages
}

/// The first line of a passage, short enough to sit in a report row.
fn opening(passage: &str) -> String {
    passage
        .lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(56)
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
