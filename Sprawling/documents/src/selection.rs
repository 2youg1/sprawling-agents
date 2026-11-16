// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::edit::Transaction;
use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: u64,
    pub focus: u64,
}

impl Selection {

    pub const fn collapsed(at: u64) -> Selection {
        Selection {
            anchor: at,
            focus: at,
        }
    }

    pub fn span(self) -> Span {
        Span::ordered(self.anchor, self.focus)
    }

    pub fn mapped(self, transaction: &Transaction) -> Selection {
        Selection {
            anchor: position_after(self.anchor, transaction),
            focus: position_after(self.focus, transaction),
        }
    }
}

fn position_after(at: u64, transaction: &Transaction) -> u64 {
    let mut added = 0_u64;
    let mut removed = 0_u64;
    for edit in transaction.edits() {
        let span = edit.span;
        if at <= span.start() {
            break;
        }
        let written = crate::span::offset(edit.bytes.len());
        if at < span.end() {
            return span
                .start()
                .saturating_add(added)
                .saturating_sub(removed)
                .saturating_add(written);
        }
        added = added.saturating_add(written);
        removed = removed.saturating_add(span.len());
    }
    at.saturating_add(added).saturating_sub(removed)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use kernel::B3Hash;

    use super::*;
    use crate::edit::Edit;

    fn save(source: &[u8], edits: &[(u64, u64, &[u8])]) -> Transaction {
        Transaction::new(
            B3Hash::digest(source),
            edits
                .iter()
                .map(|(start, end, bytes)| Edit {
                    span: Span::new(*start, *end).unwrap(),
                    bytes: bytes.to_vec(),
                })
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn a_selection_follows_the_text_it_stood_beside() {
        let source = b"0123456789";
        let saved = save(source, &[(2, 2, b"ab"), (4, 7, b"x")]);
        let after = saved.apply(source).unwrap();
        assert_eq!(after.bytes(), b"01ab23x789");
        let mapped = |at: u64| Selection::collapsed(at).mapped(&saved).anchor;
        assert_eq!(mapped(1), 1);
        assert_eq!(mapped(2), 2);
        assert_eq!(mapped(3), 5);
        assert_eq!(mapped(5), 7);
        assert_eq!(mapped(8), 8);
        assert_eq!(mapped(10), 10);
        let drawn = Selection {
            anchor: 9,
            focus: 1,
        }
        .mapped(&saved);
        assert_eq!(drawn.span(), Span::new(1, 9).unwrap());
    }
}
