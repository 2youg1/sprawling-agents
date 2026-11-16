// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Locator, Range};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    quote: String,
    at: Locator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading {

    Holds,

    OtherVersion,

    OutOfRange,

    Differs { found: String },
}

impl Citation {
    #[must_use]
    pub fn new(quote: String, at: Locator) -> Citation {
        Citation { quote, at }
    }

    #[must_use]
    pub fn quote(&self) -> &str {
        &self.quote
    }

    #[must_use]
    pub fn at(&self) -> &Locator {
        &self.at
    }

    #[must_use]
    pub fn against(&self, pinned: &Locator, bytes: &[u8]) -> Reading {
        if !same_version(&self.at, pinned) {
            return Reading::OtherVersion;
        }
        let range = range_of(&self.at);
        let Some(found) = cited_text(range, bytes).map(|text| normalised(&text)) else {
            return Reading::OutOfRange;
        };
        let quote = normalised(&self.quote);
        let holds = match range {
            Some(_) => found == quote,
            None => found.contains(&quote),
        };
        if holds {
            Reading::Holds
        } else {
            Reading::Differs { found }
        }
    }
}

fn same_version(cited: &Locator, pinned: &Locator) -> bool {
    match (cited, pinned) {
        (Locator::Cas { hash: a, .. }, Locator::Cas { hash: b, .. }) => a == b,
        (
            Locator::File {
                address: a,
                oid: a_oid,
                ..
            },
            Locator::File {
                address: b,
                oid: b_oid,
                ..
            },
        ) => a == b && a_oid == b_oid,
        (Locator::Cas { .. }, Locator::File { .. })
        | (Locator::File { .. }, Locator::Cas { .. }) => false,
    }
}

fn range_of(at: &Locator) -> Option<Range> {
    match at {
        Locator::Cas { range, .. } | Locator::File { range, .. } => *range,
    }
}

fn cited_text(range: Option<Range>, bytes: &[u8]) -> Option<String> {
    match range {
        None => std::str::from_utf8(bytes).ok().map(str::to_owned),
        Some(Range::Bytes { from, to }) => {
            let from = usize::try_from(from).ok()?;
            let end = usize::try_from(to).ok()?.checked_add(1)?;
            std::str::from_utf8(bytes.get(from..end)?)
                .ok()
                .map(str::to_owned)
        }
        Some(Range::Lines { from, to }) => {
            let skip = usize::try_from(from.checked_sub(1)?).ok()?;
            let take = usize::try_from(to.checked_sub(from)?.checked_add(1)?).ok()?;
            let lines: Vec<&str> = std::str::from_utf8(bytes)
                .ok()?
                .lines()
                .skip(skip)
                .take(take)
                .collect();
            (lines.len() == take).then(|| lines.join("\n"))
        }
    }
}

fn normalised(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use kernel::B3Hash;

    const DRAFT: &str =
        "The city keeps one ledger.\nEvery effect\n   is recorded first.\nNothing else.\n";

    fn pinned() -> Locator {
        Locator::Cas {
            hash: B3Hash::digest(DRAFT.as_bytes()),
            range: None,
        }
    }

    fn cite(quote: &str, range: Option<Range>) -> Citation {
        Citation::new(
            quote.to_owned(),
            Locator::Cas {
                hash: B3Hash::digest(DRAFT.as_bytes()),
                range,
            },
        )
    }

    #[test]
    fn a_quote_that_the_range_does_not_say_is_reported() {
        let reading = cite(
            "Every effect is recorded later.",
            Some(Range::lines(2, 3).unwrap()),
        )
        .against(&pinned(), DRAFT.as_bytes());
        assert_eq!(
            reading,
            Reading::Differs {
                found: "Every effect is recorded first.".to_owned()
            }
        );
    }

    #[test]
    fn a_quote_matches_across_line_breaks_and_indentation() {
        let reading = cite(
            "Every effect is recorded first.",
            Some(Range::lines(2, 3).unwrap()),
        )
        .against(&pinned(), DRAFT.as_bytes());
        assert_eq!(reading, Reading::Holds);
    }

    #[test]
    fn a_byte_range_is_read_closed() {
        let reading = cite("one ledger.", Some(Range::bytes(15, 25).unwrap()))
            .against(&pinned(), DRAFT.as_bytes());
        assert_eq!(reading, Reading::Holds);
    }

    #[test]
    fn a_citation_of_an_earlier_draft_does_not_hold_even_when_the_words_match() {
        let earlier = Citation::new(
            "The city keeps one ledger.".to_owned(),
            Locator::Cas {
                hash: B3Hash::digest(b"an earlier draft"),
                range: Some(Range::lines(1, 1).unwrap()),
            },
        );
        assert_eq!(
            earlier.against(&pinned(), DRAFT.as_bytes()),
            Reading::OtherVersion
        );
    }

    #[test]
    fn a_range_past_the_end_is_out_of_range() {
        let reading = cite("anything", Some(Range::lines(4, 9).unwrap()))
            .against(&pinned(), DRAFT.as_bytes());
        assert_eq!(reading, Reading::OutOfRange);
    }

    #[test]
    fn without_a_range_the_quote_may_sit_anywhere_in_the_version() {
        assert_eq!(
            cite("effect is recorded", None).against(&pinned(), DRAFT.as_bytes()),
            Reading::Holds
        );
    }
}
