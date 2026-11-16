// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::consts_external::{SECRET_SHAPES, SecretCharset};
use crate::consts_policy::SECRET_ENTROPY_MIN;

use super::hex_run::is_labelled_hex_secret;
use super::span::SecretSpan;
use crate::Address;
use std::collections::BTreeSet;
use std::sync::LazyLock;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntropyAllowlist {
    paths: BTreeSet<String>,
}

impl EntropyAllowlist {
    pub fn allows(&self, path: &str) -> bool {
        self.paths.contains(path)
    }
}

impl FromIterator<Address> for EntropyAllowlist {
    fn from_iter<T: IntoIterator<Item = Address>>(paths: T) -> Self {
        EntropyAllowlist {
            paths: paths
                .into_iter()
                .map(|path| path.as_str().to_owned())
                .collect(),
        }
    }
}

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

fn token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'+' | b'/')
}

fn mixed_alphabet(bytes: &[u8]) -> bool {
    let (mut upper, mut lower, mut digit) = (false, false, false);
    for byte in bytes {
        upper |= byte.is_ascii_uppercase();
        lower |= byte.is_ascii_lowercase();
        digit |= byte.is_ascii_digit();
    }
    upper && lower && digit
}

pub(crate) const ENTROPY_SPAN_MIN_BYTES: usize = 20;

fn log2_q10(x: u64) -> u64 {
    if x == 0 {
        return 0;
    }
    let int_part = u64::from(x.ilog2());

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

pub(super) fn entropy_millibits_per_char(bytes: &[u8]) -> u64 {
    let Ok(n) = u64::try_from(bytes.len()) else {
        return 0;
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

    sum_q10
        .saturating_mul(1000)
        .checked_div(1024)
        .and_then(|mb| mb.checked_div(n))
        .unwrap_or(0)
}

fn entropy_passes(bytes: &[u8], work: &mut Work) -> bool {
    let mb = work.entropy_of(bytes);
    let num = u64::from(SECRET_ENTROPY_MIN.num);
    let den = u64::from(SECRET_ENTROPY_MIN.den);

    mb.saturating_mul(den) >= num.saturating_mul(1000)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Work {
    pub(super) bytes_read: u64,

    pub(super) entropy_readings: u64,
}

impl Work {
    fn read(&mut self, bytes: usize) {
        self.bytes_read = self
            .bytes_read
            .saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
    }

    pub(super) fn entropy_of(&mut self, run: &[u8]) -> u64 {
        self.entropy_readings = self.entropy_readings.saturating_add(1);
        self.read(run.len());
        entropy_millibits_per_char(run)
    }
}

pub fn scan(bytes: &[u8]) -> Vec<SecretSpan> {
    scan_counting(bytes).0
}

pub(super) fn scan_counting(bytes: &[u8]) -> (Vec<SecretSpan>, Work) {
    let mut work = Work::default();
    let mut hits = find_shape_hits(bytes, &mut work);
    let mut found = Vec::new();
    let mut claimed = Claimed::new(&hits);
    let mut at = 0usize;
    while at < bytes.len() {
        if !bytes.get(at).copied().is_some_and(token_byte) {
            at = at.saturating_add(1);
            continue;
        }
        let end = bytes
            .get(at..)
            .and_then(|rest| rest.iter().position(|b| !token_byte(*b)))
            .map_or(bytes.len(), |rel| at.saturating_add(rel));
        let len = end.saturating_sub(at);
        work.read(len);
        if len >= ENTROPY_SPAN_MIN_BYTES
            && !claimed.overlaps(at, end)
            && (bytes.get(at..end).is_some_and(|run| {
                work.read(run.len());
                mixed_alphabet(run) && entropy_passes(run, &mut work)
            }) || is_labelled_hex_secret(bytes, at, end, &mut work))
        {
            found.push(SecretSpan {
                start: at,
                len,
                provider: None,
            });
        }
        at = end;
    }
    hits.append(&mut found);
    hits.sort_by_key(|h| (h.start, h.len));
    (hits, work)
}

static OPENS_A_SHAPE: LazyLock<[bool; 256]> = LazyLock::new(|| {
    let mut table = [false; 256];
    for shape in &SECRET_SHAPES {
        if let Some(slot) = shape
            .prefix
            .as_bytes()
            .first()
            .and_then(|first| table.get_mut(usize::from(*first)))
        {
            *slot = true;
        }
    }
    table
});

fn find_shape_hits(bytes: &[u8], work: &mut Work) -> Vec<SecretSpan> {
    let mut resume = [0usize; SECRET_SHAPES.len()];
    let mut hits = Vec::new();
    let opens: &[bool; 256] = &OPENS_A_SHAPE;
    work.read(bytes.len());
    for (start, first) in bytes.iter().enumerate() {
        if !opens.get(usize::from(*first)).copied().unwrap_or(false) {
            continue;
        }
        let Some(rest) = bytes.get(start..) else {
            continue;
        };
        for (shape, next) in SECRET_SHAPES.iter().zip(resume.iter_mut()) {
            let prefix = shape.prefix.as_bytes();
            if start < *next || prefix.is_empty() || !rest.starts_with(prefix) {
                continue;
            }
            let body = rest.get(prefix.len()..).map_or(0, |tail| {
                tail.iter()
                    .take_while(|b| charset_admits(shape.charset, **b))
                    .count()
            });
            work.read(body);
            let total = prefix.len().saturating_add(body);
            if total >= usize::from(shape.len.0) {
                hits.push(SecretSpan {
                    start,
                    len: total.min(usize::from(shape.len.1)),
                    provider: Some(shape.provider),
                });
            }
            *next = start.saturating_add(prefix.len());
        }
    }
    hits.sort_by_key(|h| (h.start, h.len));
    hits
}

struct Claimed<'a> {
    spans: &'a [SecretSpan],
    next: usize,

    reach: usize,
}

impl<'a> Claimed<'a> {
    fn new(spans: &'a [SecretSpan]) -> Claimed<'a> {
        Claimed {
            spans,
            next: 0,
            reach: 0,
        }
    }

    fn overlaps(&mut self, start: usize, end: usize) -> bool {
        while let Some(span) = self.spans.get(self.next).filter(|s| s.start < start) {
            self.reach = self.reach.max(span.start.saturating_add(span.len));
            self.next = self.next.saturating_add(1);
        }
        self.reach > start
            || self
                .spans
                .get(self.next..)
                .unwrap_or_default()
                .iter()
                .take_while(|s| s.start < end)
                .any(|s| s.start.saturating_add(s.len) > start)
    }
}

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
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    reason = "test code"
)]
mod tests;

#[cfg(kani)]
mod verification;
