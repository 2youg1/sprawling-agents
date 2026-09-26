// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A quote checked against the version pinned for review.
//!
//! The citation is the smallest form that can be checked mechanically:
//! the quoted words and the `cas:` or `file:` [`Locator`] range they
//! claim to come from. A verification run calls [`Citation::against`]
//! once per citation and reports every reading that is not
//! [`Reading::Holds`]; what to check and why stays with the model.

use kernel::{Locator, Range};

/// A quote and the place it claims to come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    quote: String,
    at: Locator,
}

/// What comparing one citation with the pinned version found.
///
/// An outcome rather than an error: a misquote is what a verification
/// run exists to report, not a fault for `?` to carry away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading {
    /// After normalisation, the range says exactly the quote.
    Holds,
    /// The citation names a different version from the pinned one, so
    /// matching words would prove nothing about the text under review.
    OtherVersion,
    /// The range runs past the pinned version, or lands on bytes that
    /// are not UTF-8.
    OutOfRange,
    /// The range exists and says something else; `found` is its
    /// normalised text, for the report to quote.
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

    /// Compares the quote with `bytes`, the content `pinned` resolves
    /// to. The version is judged before any text is read.
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

/// Whether two locators name the same content, whatever range each
/// carries.
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

/// The text a range selects, or `None` when it runs past the end or
/// is not UTF-8. Line ranges are 1-based and byte ranges 0-based, both
/// closed, as the Locator grammar defines them.
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

/// Line breaks and indentation are layout, not content; anything
/// further (case, punctuation) would let a quote that changed the
/// meaning pass.
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
