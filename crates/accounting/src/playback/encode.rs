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
/// # Errors
/// `E_INVALID_ARGS` when the bundle would exceed [`BUNDLE_MAX_BYTES`];
/// the recovery names the flags that narrow a selection.
pub(super) fn encode(document: &Document) -> Result<Bundle, AxError> {
    let compact = serde_json::to_string(document).map_err(|err| {
        AxError::failure(
            AxCode::InvalidArgs,
            "write a playback bundle",
            err.to_string(),
        )
        .with_recovery("export again; a bundle this build cannot serialise is a defect to report")
    })?;
    let bytes = escaped(&compact).into_bytes();
    if bytes.len() > BUNDLE_MAX_BYTES {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "write a playback bundle",
            format!("{} bytes, over {BUNDLE_MAX_BYTES}", bytes.len()),
        )
        .with_recovery("narrow the selection with --from and --through, --run or --building"));
    }
    Ok(Bundle {
        events: document.events.len(),
        bytes,
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

/// Compact JSON with the five characters that can end or break an HTML
/// script block written as escapes.
fn escaped(compact: &str) -> String {
    let mut out = String::with_capacity(compact.len());
    for character in compact.chars() {
        match character {
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            other => out.push(other),
        }
    }
    out
}
