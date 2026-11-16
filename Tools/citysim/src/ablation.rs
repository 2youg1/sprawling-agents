// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, ByteLen};

pub(crate) mod capabilities;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Capability {
    pub(crate) name: &'static str,
    pub(crate) cue: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Passage {
    pub(crate) index: u32,
    pub(crate) opening: String,
    pub(crate) removed: ByteLen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Cost {
    Untouched,

    Restated { also_said: Vec<&'static str> },

    Sole { lost: Vec<&'static str> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Charge {
    pub(crate) passage: Passage,
    pub(crate) cost: Cost,
}

#[derive(Debug, Clone)]
pub(crate) struct Ablation {
    passages: Vec<String>,
    corpus: &'static [Capability],
}

impl Ablation {
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
