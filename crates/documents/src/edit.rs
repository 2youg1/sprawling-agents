// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One save of a document: edits against the version they were made on
//! (`crates/documents/spec/Edit.lean`, D8 and D9).
//!
//! A transaction names the version it was made on, so a second writer
//! that started from the same version learns that the document moved
//! instead of writing over the first writer's change. Its edits are
//! byte spans of that version, in order and apart, so every byte outside
//! them is copied through untouched, and applying one hands back the
//! transaction that undoes it. A page sends text rather than bytes, and
//! [`save`] writes that text in the version's own encoding (D11, D12).

use kernel::{AxCode, AxError, B3Hash};
use serde::{Deserialize, Serialize};

use crate::encoding::{Encoding, Reading};
use crate::span::{Span, offset};

/// One edit as a page sends it: a stretch of the baseline's bytes, and
/// the text that takes its place (D11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TextEdit {
    pub span: Span,
    pub text: String,
}

/// Applies a page's text edits to `source`, which must be the version
/// `baseline`, writing each text in that version's encoding (D11, D12).
///
/// # Errors
/// `E_VERSION_CONFLICT` when `source` is not `baseline`, judged before
/// anything else. `E_INVALID_ARGS` when the version is not text, when an
/// edit starts or ends inside a character, when the edits are out of
/// order, overlap or reach past the end, and when the result would not
/// read as text in the same encoding. Nothing was applied in any case.
pub fn save(source: &[u8], baseline: B3Hash, edits: &[TextEdit]) -> Result<Applied, AxError> {
    if B3Hash::digest(source) != baseline {
        return Err(moved(baseline));
    }
    saved_on(source, baseline, edits)
}

/// [`save`] once the source is known to be `baseline`, so a caller that
/// already compared the digest does not hash the version twice.
pub(crate) fn saved_on(
    source: &[u8],
    baseline: B3Hash,
    edits: &[TextEdit],
) -> Result<Applied, AxError> {
    let Reading::Text(encoding) = Reading::of(source) else {
        return Err(not_text(
            "the version is not text in any encoding this city reads",
        ));
    };
    let bytes = edits
        .iter()
        .map(|edit| {
            on_characters(encoding, source, edit.span).map(|()| Edit {
                span: edit.span,
                bytes: encoding.encode(&edit.text),
            })
        })
        .collect::<Result<Vec<Edit>, AxError>>()?;
    let applied = Transaction::new(baseline, bytes)?.splice(source)?;
    if Reading::of(&applied.bytes) != Reading::Text(encoding) {
        return Err(not_text(
            "the saved bytes would not read as text in the version's own encoding",
        ));
    }
    Ok(applied)
}

/// Whether both ends of `span` fall on a character of `source`.
fn on_characters(encoding: Encoding, source: &[u8], span: Span) -> Result<(), AxError> {
    let starts = |at: u64| {
        usize::try_from(at)
            .is_ok_and(|index| index == source.len() || encoding.begins_at(source, index, 0))
    };
    if starts(span.start()) && starts(span.end()) {
        Ok(())
    } else {
        Err(not_text(&format!(
            "the edit at {}..{} starts or ends inside a character",
            span.start(),
            span.end()
        )))
    }
}

fn not_text(why: &str) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "save a document", why.to_owned())
        .with_recovery("read the document again: its answer says whether and how it reads as text")
}

fn moved(baseline: B3Hash) -> AxError {
    AxError::failure(
        AxCode::VersionConflict,
        "save a document",
        format!("the document is no longer version {baseline}"),
    )
    .with_recovery("read the document again and make the change on what it says now")
}

/// One replacement: the bytes of `span` in the baseline become `bytes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub span: Span,
    pub bytes: Vec<u8>,
}

/// Edits against one version, in order, none overlapping another.
///
/// [`Transaction::new`] is the one construction point, so edits out of
/// order are refused once rather than read two ways.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    baseline: B3Hash,
    edits: Vec<Edit>,
}

/// What applying a transaction left: the new version's bytes and
/// identity, and the transaction that takes it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    baseline: B3Hash,
    bytes: Vec<u8>,
    version: B3Hash,
    undo: Transaction,
}

impl Transaction {
    /// Edits made on the version `baseline`.
    ///
    /// Two insertions at one offset are kept in the order given; any
    /// other pair must not share a byte.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` naming the first edit that starts before the one
    /// ahead of it ends.
    pub fn new(baseline: B3Hash, edits: Vec<Edit>) -> Result<Transaction, AxError> {
        let tangled = edits
            .iter()
            .zip(edits.iter().skip(1))
            .find_map(|(earlier, later)| {
                (later.span.start() < earlier.span.end()).then_some(later.span)
            });
        if let Some(span) = tangled {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "save a document",
                format!(
                    "the edit at {}..{} starts before the edit ahead of it ends",
                    span.start(),
                    span.end()
                ),
            )
            .with_recovery("send the edits in document order, each apart from the next"));
        }
        Ok(Transaction { baseline, edits })
    }

    /// The version these edits were made on.
    pub fn baseline(&self) -> B3Hash {
        self.baseline
    }

    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }

    /// Applies the edits to `source`, which must be the baseline version.
    ///
    /// # Errors
    /// `E_VERSION_CONFLICT` when `source` is not the baseline: somebody
    /// saved since these edits were made, and their change stands.
    /// `E_INVALID_ARGS` when an edit reaches past the end of `source`.
    /// Either way nothing was applied.
    pub fn apply(&self, source: &[u8]) -> Result<Applied, AxError> {
        if B3Hash::digest(source) != self.baseline {
            return Err(moved(self.baseline));
        }
        self.splice(source)
    }

    /// Applies the edits to `source`, already known to be the baseline.
    fn splice(&self, source: &[u8]) -> Result<Applied, AxError> {
        let mut bytes = Vec::with_capacity(source.len());
        let mut undo = Vec::with_capacity(self.edits.len());
        let mut cursor = 0_usize;
        for edit in &self.edits {
            let outside = || past_the_end(edit.span, source.len());
            let (start, end) = edit.span.within(source.len()).ok_or_else(outside)?;
            bytes.extend_from_slice(source.get(cursor..start).ok_or_else(outside)?);
            let replaced = source.get(start..end).ok_or_else(outside)?;
            let from = offset(bytes.len());
            bytes.extend_from_slice(&edit.bytes);
            undo.push(Edit {
                span: Span::ordered(from, offset(bytes.len())),
                bytes: replaced.to_vec(),
            });
            cursor = end;
        }
        bytes.extend_from_slice(
            source
                .get(cursor..)
                .ok_or_else(|| past_the_end(Span::at(offset(cursor)), source.len()))?,
        );
        let version = B3Hash::digest(&bytes);
        Ok(Applied {
            baseline: self.baseline,
            bytes,
            version,
            undo: Transaction {
                baseline: version,
                edits: undo,
            },
        })
    }
}

impl Applied {
    /// The version the transaction was applied on.
    pub fn baseline(&self) -> B3Hash {
        self.baseline
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The identity of the version the transaction made.
    pub fn version(&self) -> B3Hash {
        self.version
    }

    /// The transaction that takes this version back to the baseline,
    /// made on this version (D9).
    pub fn undo(&self) -> &Transaction {
        &self.undo
    }
}

fn past_the_end(span: Span, len: usize) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "save a document",
        format!(
            "the edit at {}..{} reaches past the end of {len} bytes",
            span.start(),
            span.end()
        ),
    )
    .with_recovery("read the document again: the edit was made on a longer version")
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
