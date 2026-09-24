// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One turn's exchange, and the one moment it may be replaced.
//!
//! An exchange is the assistant message that asked for the calls and the
//! answers it got: everything one turn adds to the window. The turn
//! layer collects it as its wave lands, and `runtime::fork` collects the
//! same value from the same records, because neither half is a
//! conversation without the other.
//!
//! Three rules hold this module up.
//!
//! **The snapshot is replaced at the turn's closing boundary and
//! nowhere else.** The compaction runs once, after the whole wave is
//! down, at the boundary `Turn<Recording>::record` closes at. Mid-wave
//! is the one forbidden moment: a compactor meeting a half-landed wave
//! would group what a boundary groups once, and `runtime::fork` — which
//! rebuilds one turn at a time from the ledger — would answer different
//! bytes for the same history. A rebuild that cannot reproduce a live
//! window is how a fork drifts from the conversation it branched off.
//!
//! **The verdict is `plan`'s.** One comparison decides whether the
//! exchange is over budget at all: the total of its texts against the
//! budget, whose value has one home in `consts_policy`. Everything
//! after that is `plan`'s — each text is handed to [`super::plan`]
//! against its even share, the same "does it fit, and what happens when
//! it does not" that every other shortening in `crate::compaction`
//! takes its orders from.
//!
//! **What must leave whole stays whole.** `MustOffload` means a text
//! whose shape is destroyed by cutting, and this door holds no store to
//! take it — the pipeline's landing is where eviction happens, with the
//! tee in front of it. So the text stays: what this door hands the window
//! is the bytes the record already carries, and a result the landing
//! pipeline did shorten has its original in the content store behind the
//! account that names it. A thinking block is never touched at all: it
//! travels with a signature over its exact bytes, and its redacted form is
//! sealed.

use kernel::consts_policy::EXCHANGE_BUDGET_BYTES;
use kernel::{AxCode, AxError, ByteLen, ContentBlock};

use super::{Shrink, detect, plan, shorten};

/// One turn's exchange: the assistant reply and the wave's results.
///
/// The two vectors always travel together and are never chosen
/// independently — a reply that asked for calls and the answers to them
/// are one exchange — so they travel as one value, collected by the turn
/// as its wave lands and rebuilt by `runtime::fork` from the records
/// that wave wrote.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Exchange {
    assistant: Vec<ContentBlock>,
    results: Vec<ContentBlock>,
}

impl Exchange {
    #[must_use]
    pub fn new() -> Exchange {
        Exchange::default()
    }

    /// Appends the assistant reply's blocks. One call per turn: the
    /// reply precedes its wave in every conversation and in every
    /// rebuild of one.
    pub fn push_assistant(&mut self, content: Vec<ContentBlock>) {
        self.assistant.extend(content);
    }

    /// Appends one wave result, as the model reads it back.
    pub fn push_result(&mut self, block: ContentBlock) {
        self.results.push(block);
    }

    #[must_use]
    pub fn assistant(&self) -> &[ContentBlock] {
        &self.assistant
    }

    #[must_use]
    pub fn results(&self) -> &[ContentBlock] {
        &self.results
    }

    /// Replaces this snapshot with the compacted one. The turn's closing
    /// boundary calls this once per turn; `runtime::fork` calls it at the
    /// same boundary of the same turn, which is what makes a rebuild
    /// byte-identical to the window the run actually sent.
    ///
    /// # Errors
    /// Reports `E_INVALID_ARGS` when a text or the exchange cannot be
    /// counted in a `u64` — a size no tool this city ships can produce.
    pub fn compact(&mut self) -> Result<(), AxError> {
        let mut texts = Vec::new();
        for block in self.assistant.iter_mut().chain(self.results.iter_mut()) {
            match block {
                ContentBlock::Text { text } => texts.push(text),
                ContentBlock::ToolResult { content, .. } => texts.push(content),
                ContentBlock::Thinking { .. }
                | ContentBlock::RedactedThinking { .. }
                | ContentBlock::ToolUse { .. }
                | ContentBlock::Image(_) => {}
            }
        }
        compact_texts(texts)
    }
}

/// The exchange's texts, each planned against its even share of the
/// budget.
///
/// The total is the trigger and the share is the budget; both are read
/// against [`super::plan`] rather than re-decided here. An empty text
/// counts for neither — it occupies no window and would only dilute the
/// shares of texts that do.
fn compact_texts(texts: Vec<&mut String>) -> Result<(), AxError> {
    let budget = ByteLen::new(EXCHANGE_BUDGET_BYTES);
    let mut total = 0u64;
    let mut count = 0u64;
    for text in &texts {
        if text.is_empty() {
            continue;
        }
        total = total.checked_add(bytes(text)?).ok_or_else(uncountable)?;
        count = count.checked_add(1).ok_or_else(uncountable)?;
    }
    if count == 0 || total <= budget.get() {
        return Ok(());
    }
    let share = budget.get().checked_div(count).ok_or_else(uncountable)?;
    let share = ByteLen::new(share);
    for text in texts {
        if text.is_empty() {
            continue;
        }
        let size = ByteLen::new(bytes(text)?);
        match plan(detect(text), size, share) {
            Shrink::Keep | Shrink::MustOffload => {}
            Shrink::Cut(strategy) => {
                let cut = shorten(text, strategy, share);
                *text = cut.text;
            }
        }
    }
    Ok(())
}

fn bytes(text: &str) -> Result<u64, AxError> {
    u64::try_from(text.len()).map_err(|_| uncountable())
}

fn uncountable() -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "compact one turn's exchange",
        "a window text larger than a u64 byte count",
    )
    .with_recovery(
        "report this against runtime::compaction::exchange: no tool this city ships \
         can put a text this size in the window",
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]
mod tests;
