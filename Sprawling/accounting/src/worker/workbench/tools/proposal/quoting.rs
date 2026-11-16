// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a run's quote makes of a document as the city holds it: the
//! version read at the call, and the byte stretch of it the quote is
//! (`crates/accounting/spec/Worker/Workbench/Tools.lean` §8-30, `crates/documents/Spec.lean` D35).

use std::io::Read as _;

use documents::{Encoding, Reading};
use kernel::event::record::ProposalOffered;
use kernel::{AxCode, AxError, B3Hash};
use runtime::tools::Named;

use super::{ACTION, Offering, refused};

/// The `proposal_offered` line `asked` makes of the document as the
/// city holds it now (documents D35), read through `reader`.
pub(super) fn offered(
    reader: &runtime::BoundReader,
    asked: &Offering,
) -> Result<ProposalOffered, AxError> {
    if asked.old.is_empty() {
        return Err(refused(
            "`old` is empty".to_owned(),
            "quote the text the change replaces; to add text, quote the sentence it goes \
             beside and repeat that sentence in `new`",
        ));
    }
    if asked.old == asked.new {
        return Err(refused(
            "`new` is the same as `old`".to_owned(),
            "suggest text that differs from what the document says",
        ));
    }
    if kernel::Locator::parse(&asked.path).is_ok() {
        return Err(refused(
            format!("`{}` is a locator", asked.path),
            "name the document by its path: a proposal is about the city's copy as it stands",
        ));
    }
    let mut opened = reader.open(&asked.path, ACTION)?;
    let Named::File(doc) = opened.named().clone() else {
        return Err(refused(
            format!("`{}` is a stored block, not a document", asked.path),
            "name the document by its path",
        ));
    };
    let mut bytes = Vec::new();
    opened.read_to_end(&mut bytes).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            ACTION,
            format!("{}: {err}", asked.path),
        )
        .with_recovery("the User has to make the file readable")
    })?;
    let Reading::Text(encoding) = Reading::of(&bytes) else {
        return Err(refused(
            format!("`{}` does not read as text", asked.path),
            "propose changes only to a text document",
        ));
    };
    let (start, end) = stretch(&encoding.decode(&bytes)?, &asked.old, encoding)?;
    Ok(ProposalOffered {
        doc,
        baseline: B3Hash::digest(&bytes),
        start,
        end,
        before: asked.old.clone(),
        after: asked.new.clone(),
    })
}

/// Where `old` lies in `text`, the whole of a version read in
/// `encoding`, as a byte interval of that version: in either UTF-8 the
/// text's bytes are the version's (documents D5), in UTF-16 each code
/// unit is two bytes.
fn stretch(text: &str, old: &str, encoding: Encoding) -> Result<(u64, u64), AxError> {
    let mut places = text.match_indices(old).map(|(at, _)| at);
    let (Some(at), None) = (places.next(), places.next()) else {
        return Err(match text.matches(old).count() {
            0 => refused(
                "`old` is not in the document as the city holds it".to_owned(),
                "read the document and quote it exactly; a run under review quotes the city's \
                 copy, not its own tree's",
            ),
            times => refused(
                format!("`old` appears {times} times in the document"),
                "quote more of the text around the change, so that it appears once",
            ),
        });
    };
    let width = |piece: &str| match encoding {
        Encoding::Utf8 | Encoding::Utf8Bom => piece.len(),
        Encoding::Utf16Le | Encoding::Utf16Be => piece.encode_utf16().count().saturating_mul(2),
    };
    let before = text.get(..at).ok_or_else(|| {
        refused(
            "`old` does not begin at a character".to_owned(),
            "quote whole characters",
        )
    })?;
    let start = width(before);
    let end = start.checked_add(width(old)).ok_or_else(|| {
        refused(
            "the stretch ends past what a document can hold".to_owned(),
            "quote a shorter stretch",
        )
    })?;
    Ok((offset(start)?, offset(end)?))
}

/// A byte count as the offset a line carries.
fn offset(count: usize) -> Result<u64, AxError> {
    u64::try_from(count).map_err(|_| {
        refused(
            format!("offset {count} does not fit a line"),
            "quote a shorter stretch",
        )
    })
}
