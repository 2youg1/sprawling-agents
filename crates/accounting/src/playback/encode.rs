// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one encoding of a playback bundle (`crates/accounting/spec/Playback.lean` §8-12).
//!
//! Compact JSON in the field order `document` declares, with `<`, `>`,
//! `&`, U+2028 and U+2029 written as `\u` escapes. Those characters only
//! ever occur inside JSON strings, so the escape changes no value, and a
//! bundle's bytes can sit inside an HTML `<script>` block as they are: a
//! log line holding `</script>` cannot end the block early. The digest
//! is taken over these bytes, and reading a bundle back re-encodes it
//! and requires the same bytes, which is how a duplicate key, extra
//! whitespace or a reordered field is refused.

use kernel::{AxCode, AxError, B3Hash};

use super::document::Document;
use super::{BUNDLE_MAX_BYTES, SCHEMA};

/// A playback bundle: canonical bytes, built only by [`encode`] and
/// [`decode`], so every value of this type is a bundle this build can
/// read back to the same bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    bytes: Vec<u8>,
    events: usize,
}

impl Bundle {
    /// The canonical bytes, safe to embed in HTML as they are.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// BLAKE3 over [`Bundle::bytes`].
    #[must_use]
    pub fn digest(&self) -> B3Hash {
        B3Hash::digest(&self.bytes)
    }

    /// How many selected events the bundle holds.
    #[must_use]
    pub fn events(&self) -> usize {
        self.events
    }
}

/// Encodes a document, refusing one whose bytes exceed the ceiling.
///
/// serde writes through [`Escaping`] straight into the bundle's one
/// buffer, so no compact copy and no escaped copy of the whole document
/// exist beside it, and a document past the ceiling stops being written
/// where it crosses it.
///
/// # Errors
/// `E_INVALID_ARGS` when the bundle would exceed [`BUNDLE_MAX_BYTES`];
/// the recovery names the flags that narrow a selection.
pub(super) fn encode(document: &Document) -> Result<Bundle, AxError> {
    let mut escaping = Escaping::within(BUNDLE_MAX_BYTES);
    match serde_json::to_writer(&mut escaping, document) {
        Ok(()) => {}
        // The writer's only failure is the ceiling.
        Err(over) if over.is_io() => {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "write a playback bundle",
                format!("over {BUNDLE_MAX_BYTES} bytes"),
            )
            .with_recovery("narrow the selection with --from and --through, --run or --building"));
        }
        Err(err) => {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "write a playback bundle",
                err.to_string(),
            )
            .with_recovery(
                "export again; a bundle this build cannot serialise is a defect to report",
            ));
        }
    }
    Ok(Bundle {
        events: document.events.len(),
        bytes: escaping.finish(),
    })
}

/// Reads bytes back into a document, requiring that they are exactly
/// what [`encode`] writes for it.
///
/// # Errors
/// `E_INVALID_ARGS` for bytes over the ceiling, a bundle of another
/// schema (named, with the build to read it with), bytes that are not a
/// bundle of this schema's shape, and bytes that are not canonical.
pub(super) fn decode(bytes: &[u8]) -> Result<(Document, Bundle), AxError> {
    let refuse = |subject: String| {
        AxError::failure(AxCode::InvalidArgs, "read a playback bundle", subject).with_recovery(
            "check the file sprawling playback export wrote, byte for byte; a bundle that was \
             re-indented or edited by hand is not one",
        )
    };
    if bytes.len() > BUNDLE_MAX_BYTES {
        return Err(refuse(format!(
            "{} bytes, over {BUNDLE_MAX_BYTES}",
            bytes.len()
        )));
    }
    let named: Schema = serde_json::from_slice(bytes).map_err(|err| refuse(err.to_string()))?;
    if named.schema != SCHEMA {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "read a playback bundle",
            format!(
                "schema '{}', where this build reads '{SCHEMA}'",
                named.schema
            ),
        )
        .with_recovery(
            "check it with the build that exported it, or export it again with this one",
        ));
    }
    let document: Document =
        serde_json::from_slice(bytes).map_err(|err| refuse(err.to_string()))?;
    let bundle = encode(&document)?;
    if bundle.bytes != bytes {
        return Err(refuse(
            "the bytes are not the canonical encoding of what they hold".to_owned(),
        ));
    }
    Ok((document, bundle))
}

/// The one key a bundle of any schema carries, read before the rest so a
/// bundle of another schema is named as one instead of failing on a
/// field.
#[derive(serde::Deserialize)]
struct Schema {
    schema: String,
}

/// A writer that takes compact JSON and writes it with the five
/// characters that can end or break an HTML script block as escapes,
/// refusing any byte past its ceiling.
///
/// `<`, `>` and `&` are one byte each; U+2028 and U+2029 are the three
/// bytes `E2 80 A8` and `E2 80 A9`, which serde may hand over across two
/// writes, so a prefix of them is held until the byte that decides it
/// arrives.
struct Escaping {
    out: Vec<u8>,
    ceiling: usize,
    held: Held,
}

/// The prefix of a line or paragraph separator read so far.
#[derive(Clone, Copy)]
enum Held {
    Nothing,
    E2,
    E280,
}

impl Escaping {
    fn within(ceiling: usize) -> Escaping {
        Escaping {
            out: Vec::new(),
            ceiling,
            held: Held::Nothing,
        }
    }

    /// The bytes written, a held prefix that no separator completed
    /// written as it came.
    fn finish(self) -> Vec<u8> {
        self.out
    }
}

impl std::io::Write for Escaping {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.out.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use std::io::Write as _;

    use proptest::prelude::*;

    use super::Escaping;

    /// What the writer must produce for `text` written whole: every
    /// character as it is, the five as their `\u` escapes.
    fn escaped(text: &str) -> String {
        text.chars()
            .map(|character| match character {
                '<' => "\\u003c".to_owned(),
                '>' => "\\u003e".to_owned(),
                '&' => "\\u0026".to_owned(),
                '\u{2028}' => "\\u2028".to_owned(),
                '\u{2029}' => "\\u2029".to_owned(),
                other => other.to_string(),
            })
            .collect()
    }

    proptest! {
        /// However the bytes are cut into writes, the output is the text
        /// written whole with its five characters escaped: a separator
        /// split between two writes is still recognised.
        #[test]
        fn escaping_does_not_depend_on_where_the_writes_are_cut(
            text in "[a-z<>&\u{2028}\u{2029}\u{2027}\u{e280}é]{0,40}",
            cuts in proptest::collection::vec(0usize..48, 0..6),
        ) {
            let bytes = text.as_bytes();
            let mut points: Vec<usize> = cuts.into_iter().map(|cut| cut.min(bytes.len())).collect();
            points.push(bytes.len());
            points.sort_unstable();
            let mut escaping = Escaping::within(usize::MAX);
            let mut from = 0;
            for to in points {
                escaping.write_all(bytes.get(from..to).unwrap()).unwrap();
                from = to;
            }
            prop_assert_eq!(String::from_utf8(escaping.finish()).unwrap(), escaped(&text));
        }
    }

    /// Past its ceiling the writer refuses, and holds no more than the
    /// ceiling.
    #[test]
    fn the_writer_stops_at_its_ceiling() {
        let mut escaping = Escaping::within(10);
        let refused = escaping.write_all(b"<<<<");
        assert!(refused.is_err());
        assert!(escaping.finish().len() <= 10);
    }
}
