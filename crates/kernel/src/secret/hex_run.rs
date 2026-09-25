// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Hex-run detection: a pure-hex run is reported only when it is the
//! value of a credential name (kernel-SPEC §8-25).

use super::scan::{entropy_millibits_per_char, names_a_credential};

/// Shortest hex run reported: 32 hex chars carry 128 bits, the smallest
/// key size in common use. Moves with kernel-SPEC §14 only.
pub(crate) const HEX_SPAN_MIN_BYTES: usize = 32;

/// Per-char entropy floor for a hex run, in millibits. Every random
/// sample of 32 chars or more in the SPEC's measurement reads above it,
/// and it sits above the 3000 that eight evenly repeated symbols give.
pub(crate) const HEX_ENTROPY_MIN_MILLIBITS: u64 = 3100;

/// Whether the token run `bytes[start..end]` is a hex secret: pure hex,
/// at least [`HEX_SPAN_MIN_BYTES`] long, at least
/// [`HEX_ENTROPY_MIN_MILLIBITS`] per char, and the value of a name that
/// [`names_a_credential`] accepts. The label is what separates a random
/// key from a blake3 or git oid, whose entropy reads the same.
pub(super) fn is_labelled_hex_secret(bytes: &[u8], start: usize, end: usize) -> bool {
    let Some(run) = bytes.get(start..end) else {
        return false;
    };
    run.len() >= HEX_SPAN_MIN_BYTES
        && run.iter().all(u8::is_ascii_hexdigit)
        && entropy_millibits_per_char(run) >= HEX_ENTROPY_MIN_MILLIBITS
        && bytes
            .get(..start)
            .and_then(label_before)
            .is_some_and(names_a_credential)
}

/// The name a value follows: the name, optional blanks and quotes, one
/// `=` or `:`, then optional blanks and quotes up to the value.
fn label_before(head: &[u8]) -> Option<&str> {
    let (separator, head) = without_padding(head).split_last()?;
    if !matches!(separator, b'=' | b':') {
        return None;
    }
    let head = without_padding(head);
    let name_start = head
        .iter()
        .rposition(|b| !(b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.')))
        .map_or(0, |at| at.saturating_add(1));
    let name = head.get(name_start..).filter(|name| !name.is_empty())?;
    // Every byte of `name` is ASCII, so the conversion cannot fail.
    std::str::from_utf8(name).ok()
}

fn without_padding(head: &[u8]) -> &[u8] {
    let kept = head
        .iter()
        .rposition(|b| !(b.is_ascii_whitespace() || matches!(b, b'"' | b'\'')))
        .map_or(0, |at| at.saturating_add(1));
    head.get(..kept).unwrap_or(head)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::string_slice, reason = "test code")]
mod tests {
    use crate::secret::scan;

    /// Probes are assembled at runtime so this file never holds a
    /// scannable span (`xtask secret` runs on it).
    fn hex40() -> String {
        ["9f3c1a7e5b", "20d48c6f1e", "a7b3905d2c", "e81f64b07a"].concat()
    }

    #[test]
    fn a_labelled_hex_key_is_reported() {
        let key = hex40();
        for text in [
            format!("HMAC_KEY={key}"),
            format!("api_token: \"{key}\""),
            format!("{{\"secret\": \"{key}\"}}"),
        ] {
            let hits = scan(text.as_bytes());
            assert_eq!(hits.len(), 1, "{text}");
            let hit = &hits[0];
            assert_eq!(
                &text.as_bytes()[hit.start..hit.start + hit.len],
                key.as_bytes()
            );
            assert_eq!(hit.provider, None);
        }
    }

    #[test]
    fn a_hex_run_without_a_credential_name_is_not_reported() {
        let oid = hex40();
        let hash = [hex40(), hex40()[..24].to_owned()].concat();
        for text in [
            format!("commit {oid}"),
            format!("oid={oid}"),
            format!("prev: {hash}"),
            format!("KEY={}", "0f1e2d3c".repeat(5)),
            format!("KEY={}", &oid[..28]),
        ] {
            assert!(scan(text.as_bytes()).is_empty(), "{text}");
        }
    }
}
