// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::scan::{Work, names_a_credential};

pub(crate) const HEX_SPAN_MIN_BYTES: usize = 32;

pub(crate) const HEX_ENTROPY_MIN_MILLIBITS: u64 = 3100;

pub(super) fn is_labelled_hex_secret(
    bytes: &[u8],
    start: usize,
    end: usize,
    work: &mut Work,
) -> bool {
    let Some(run) = bytes.get(start..end) else {
        return false;
    };
    run.len() >= HEX_SPAN_MIN_BYTES
        && run.iter().all(u8::is_ascii_hexdigit)
        && bytes
            .get(..start)
            .and_then(label_before)
            .is_some_and(names_a_credential)
        && work.entropy_of(run) >= HEX_ENTROPY_MIN_MILLIBITS
}

pub(super) fn label_before(head: &[u8]) -> Option<&str> {
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
