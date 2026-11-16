// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::consts_policy::EXCHANGE_BUDGET_BYTES;
use kernel::{AxCode, AxError, ByteLen, ContentBlock};

use super::{Shrink, detect, plan, shorten};

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

    pub fn push_assistant(&mut self, content: Vec<ContentBlock>) {
        self.assistant.extend(content);
    }

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

    pub fn compact(&mut self) -> Result<std::collections::BTreeSet<String>, AxError> {
        let mut texts = Vec::new();
        for block in self.assistant.iter_mut().chain(self.results.iter_mut()) {
            match block {
                ContentBlock::Text { text } => texts.push((None, text)),
                ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    ..
                } => texts.push((Some(tool_use_id.as_str()), content)),
                ContentBlock::Thinking { .. }
                | ContentBlock::RedactedThinking { .. }
                | ContentBlock::ToolUse { .. }
                | ContentBlock::Image(_) => {}
            }
        }
        compact_texts(texts)
    }
}

fn compact_texts(
    texts: Vec<(Option<&str>, &mut String)>,
) -> Result<std::collections::BTreeSet<String>, AxError> {
    let budget = ByteLen::new(EXCHANGE_BUDGET_BYTES);
    let mut total = 0u64;
    let mut count = 0u64;
    for (_, text) in &texts {
        if text.is_empty() {
            continue;
        }
        total = total.checked_add(bytes(text)?).ok_or_else(uncountable)?;
        count = count.checked_add(1).ok_or_else(uncountable)?;
    }
    if count == 0 || total <= budget.get() {
        return Ok(std::collections::BTreeSet::new());
    }
    let share = budget.get().checked_div(count).ok_or_else(uncountable)?;
    let share = ByteLen::new(share);
    let mut shortened = std::collections::BTreeSet::new();
    for (id, text) in texts {
        if text.is_empty() {
            continue;
        }
        let size = ByteLen::new(bytes(text)?);
        match plan(detect(text), size, share) {
            Shrink::Keep | Shrink::MustOffload => {}
            Shrink::Cut(strategy) => {
                let cut = shorten(text, strategy, share);
                if cut.text.len() < text.len()
                    && let Some(id) = id
                {
                    shortened.insert(id.to_owned());
                }
                *text = cut.text;
            }
        }
    }
    Ok(shortened)
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
