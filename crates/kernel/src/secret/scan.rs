// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Secret scanning: shape-table-first, entropy-second, float-free.

use crate::consts_external::{SECRET_SHAPES, SecretCharset};
use crate::consts_policy::SECRET_ENTROPY_MIN;

use super::span::SecretSpan;

fn charset_admits(charset: SecretCharset, byte: u8) -> bool {
    match charset {
        SecretCharset::Base62 => byte.is_ascii_alphanumeric(),
        SecretCharset::Base64Url => {
            byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b'='
        }
        SecretCharset::HexLower => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
        SecretCharset::UpperBase36 => byte.is_ascii_uppercase() || byte.is_ascii_digit(),
    }
}

/// Entropy-detector alphabet: the union of common token charsets.
/// `=` is deliberately absent: as base64 padding it only trails (the
/// pre-padding run still crosses the length floor), while as an
/// assignment sign it welds two identifiers into one fake token
/// (`NAME=value` — the S2.12 false-positive class).
fn token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'+' | b'/')
}

/// The entropy detector only fires on mixed-alphabet runs (upper AND
/// lower AND digit): unknown-shape API keys are mixed-case base62/64,
/// while the city's own artifacts — blake3 hex, uuids, digit runs — are
/// single-case by construction and must not light up every ledger line.
/// An all-lowercase secret evades this detector; the shape table stays
/// the primary net and rotation the last remedy (7.1).
fn mixed_alphabet(bytes: &[u8]) -> bool {
    let has_upper = bytes.iter().any(u8::is_ascii_uppercase);
    let has_lower = bytes.iter().any(u8::is_ascii_lowercase);
    let has_digit = bytes.iter().any(u8::is_ascii_digit);
    has_upper && has_lower && has_digit
}

/// Minimum span for the entropy detector: mainstream API keys start
/// around 20 chars; anything shorter is prose noise. Internal affair —
/// changing it changes recall, so it moves with this SPEC only.
pub(crate) const ENTROPY_SPAN_MIN_BYTES: usize = 20;

/// log2 in Q10 fixed point (10 fractional bits), shift-and-square.
/// Zero input returns zero (callers guard); the loop bound is the
/// constant 10, so termination is by construction (V5).
fn log2_q10(x: u64) -> u64 {
    if x == 0 {
        return 0;
    }
    let int_part = u64::from(x.ilog2());
    // Normalize the mantissa into [1, 2) as Q32 in u128.
    let mantissa_q32: u128 = (u128::from(x) << 32) >> x.ilog2();
    let mut y = mantissa_q32;
    let mut frac: u64 = 0;
    for _ in 0..10 {
        y = (y.saturating_mul(y)) >> 32;
        frac <<= 1;
        if y >= (2u128 << 32) {
            frac |= 1;
            y >>= 1;
        }
    }
    (int_part << 10) | frac
}

/// Shannon entropy per char in millibits (1/1000 bit). Saturation can
/// only over-approximate, which errs toward detection — the recoverable
/// direction (entrance replaces, never refuses).
fn entropy_millibits_per_char(bytes: &[u8]) -> u64 {
    let Ok(n) = u64::try_from(bytes.len()) else {
        return 0; // unreachable on real targets; zero reads as low entropy
    };
    if n == 0 {
        return 0;
    }
    let mut counts = [0u64; 256];
    for byte in bytes {
        let slot = counts.get_mut(usize::from(*byte));
        if let Some(c) = slot {
            *c = c.saturating_add(1);
        }
    }
    let log_n = log2_q10(n);
    let mut sum_q10: u64 = 0;
    for count in counts {
        if count == 0 {
            continue;
        }
        let term = count.saturating_mul(log_n.saturating_sub(log2_q10(count)));
        sum_q10 = sum_q10.saturating_add(term);
    }
    // Q10 -> millibits: * 1000 / 1024, then / n for the per-char figure.
    sum_q10
        .saturating_mul(1000)
        .checked_div(1024)
        .and_then(|mb| mb.checked_div(n))
        .unwrap_or(0)
}

fn entropy_passes(bytes: &[u8]) -> bool {
    let mb = entropy_millibits_per_char(bytes);
    let num = u64::from(SECRET_ENTROPY_MIN.num);
    let den = u64::from(SECRET_ENTROPY_MIN.den);
    // mb >= (num / den) * 1000  <=>  mb * den >= num * 1000
    mb.saturating_mul(den) >= num.saturating_mul(1000)
}

fn find_shape_hits(bytes: &[u8]) -> Vec<SecretSpan> {
    let mut hits = Vec::new();
    for shape in &SECRET_SHAPES {
        let prefix = shape.prefix.as_bytes();
        if prefix.is_empty() || bytes.len() < prefix.len() {
            continue;
        }
        let mut at = 0usize;
        while let Some(window) = bytes.get(at..) {
            let Some(rel) = window.windows(prefix.len()).position(|w| w == prefix) else {
                break;
            };
            let start = at.saturating_add(rel);
            let body_start = start.saturating_add(prefix.len());
            let mut end = body_start;
            while bytes
                .get(end)
                .is_some_and(|b| charset_admits(shape.charset, *b))
            {
                end = end.saturating_add(1);
            }
            let total = end.saturating_sub(start);
            let (min_len, max_len) = (usize::from(shape.len.0), usize::from(shape.len.1));
            if total >= min_len {
                hits.push(SecretSpan {
                    start,
                    len: total.min(max_len),
                    provider: Some(shape.provider),
                });
            }
            at = body_start;
        }
    }
    hits.sort_by_key(|h| (h.start, h.len));
    hits
}

fn overlaps(a: &SecretSpan, start: usize, len: usize) -> bool {
    let a_end = a.start.saturating_add(a.len);
    let b_end = start.saturating_add(len);
    a.start < b_end && start < a_end
}

/// Custody's detection half. Shape table first;
/// entropy second over token runs of at least [`ENTROPY_SPAN_MIN_BYTES`]
/// that no shape already claimed. Pure, total, deterministic.
pub fn scan(bytes: &[u8]) -> Vec<SecretSpan> {
    let mut hits = find_shape_hits(bytes);
    let mut at = 0usize;
    while at < bytes.len() {
        if !bytes.get(at).copied().is_some_and(token_byte) {
            at = at.saturating_add(1);
            continue;
        }
        let mut end = at;
        while bytes.get(end).copied().is_some_and(token_byte) {
            end = end.saturating_add(1);
        }
        let len = end.saturating_sub(at);
        if len >= ENTROPY_SPAN_MIN_BYTES
            && !hits.iter().any(|h| overlaps(h, at, len))
            && bytes.get(at..end).is_some_and(mixed_alphabet)
            && bytes.get(at..end).is_some_and(entropy_passes)
        {
            hits.push(SecretSpan {
                start: at,
                len,
                provider: None,
            });
        }
        at = end;
    }
    hits.sort_by_key(|h| (h.start, h.len));
    hits
}

/// Whether a *name* reads as the name of a credential.
///
/// The other half of this module judges bytes that might be a secret;
/// this half judges a label that would sit in front of one, which is
/// what a configuration file offers when it declares an environment
/// variable. Both live here so "what looks like a credential" has one
/// authority rather than two.
///
/// Case-insensitive substring match against
/// [`crate::consts_policy::CREDENTIAL_NAME_MARKERS`]. Over-inclusive on
/// purpose: the caller refuses with an alternative, and a false refusal
/// costs a person one line of configuration while a missed one leaks a
/// key into every child process the run starts.
#[must_use]
pub fn names_a_credential(name: &str) -> bool {
    let folded = name.to_ascii_lowercase();
    crate::consts_policy::CREDENTIAL_NAME_MARKERS
        .iter()
        .any(|marker| folded.contains(marker))
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
    use super::*;
    use proptest::prelude::*;
    #[test]
    fn known_provider_shapes_are_found_with_offsets_only() {
        let text = format!("config = \"sk-ant-{}\" # key", "a1B2c3D4e5".repeat(9));
        let hits = scan(text.as_bytes());
        assert!(!hits.is_empty());
        let hit = &hits[0];
        assert_eq!(hit.provider, Some("anthropic"));
        let prefix_end = hit.start.checked_add(7).unwrap();
        assert_eq!(&text.as_bytes()[hit.start..prefix_end], b"sk-ant-");
    }

    #[test]
    fn high_entropy_tokens_hit_and_prose_does_not() {
        // The probe token is assembled at runtime so the source file
        // itself never holds a scannable span (xtask secret runs on us).
        let token = ["kJ8vQ2xR9m", "W4nZ7pL3sT", "6yB1cD5fG0", "hN8aE2iU4o"].concat();
        let noisy = format!("token = {token} ok");
        let hits = scan(noisy.as_bytes());
        assert!(hits.iter().any(|h| h.provider.is_none()));
        let prose = b"the quick brown fox jumps over the lazy dog again and again";
        assert!(scan(prose).is_empty());
        let repeated = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        assert!(scan(repeated).is_empty(), "zero entropy never hits");
    }

    #[test]
    fn city_native_artifacts_do_not_light_up() {
        // blake3 hex64: all-lowercase hex — no mixed alphabet, no hit.
        let hex = format!("prev: {}", "ab".repeat(32));
        assert!(scan(hex.as_bytes()).is_empty());
        // uuid: lowercase hex + hyphens.
        let uuid = b"run: 0198f6a2-7c4a-7bbb-9d1e-000000000001";
        assert!(scan(uuid).is_empty());
        // digit runs (timestamps, sizes).
        let digits = b"t: 17555443211234567890123";
        assert!(scan(digits).is_empty());
    }

    #[test]
    fn shape_hits_swallow_overlapping_entropy_hits() {
        let text = format!("sk-ant-{}", "a1B2c3D4e5".repeat(9));
        let hits = scan(text.as_bytes());
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].provider, Some("anthropic"));
    }

    #[test]
    fn log2_q10_matches_known_points() {
        assert_eq!(log2_q10(1), 0);
        assert_eq!(log2_q10(2), 1 << 10);
        assert_eq!(log2_q10(4), 2 << 10);
        // log2(3) = 1.58496...; Q10 => 1623.0 => 1623 (truncated toward 0)
        let three = log2_q10(3);
        assert!((1622..=1624).contains(&three), "{three}");
    }

    #[test]
    fn uniform_bytes_have_full_entropy() {
        // 64 distinct byte values once each: log2(64) = 6 bits/char.
        let bytes: Vec<u8> = (0u8..64).collect();
        let mb = entropy_millibits_per_char(&bytes);
        assert!((5900..=6000).contains(&mb), "{mb}");
    }

    proptest! {
        /// Kani mirror: total on arbitrary inputs, spans in bounds.
        #[test]
        fn scan_is_total_and_in_bounds(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
            for hit in scan(&bytes) {
                prop_assert!(hit.start.checked_add(hit.len).unwrap() <= bytes.len());
            }
        }
    }
}

#[cfg(kani)]
mod verification {
    //! V5: the entropy judgment terminates and never panics. The loop
    //! bounds are constants; kani proves absence of arithmetic panics for
    //! arbitrary short inputs.

    use super::*;

    #[kani::proof]
    fn entropy_is_total_on_short_inputs() {
        let len: usize = kani::any();
        kani::assume(len <= 4);
        let mut bytes = [0u8; 4];
        for slot in bytes.iter_mut() {
            *slot = kani::any();
        }
        let _ = entropy_millibits_per_char(&bytes[..len]);
    }

    #[kani::proof]
    fn log2_q10_is_total() {
        let x: u64 = kani::any();
        let _ = log2_q10(x);
    }
}
