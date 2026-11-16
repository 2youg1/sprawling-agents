// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::error::{AxCode, AxError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct B3Hash([u8; 32]);

impl B3Hash {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        B3Hash(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn digest(bytes: &[u8]) -> Self {
        B3Hash(*blake3::hash(bytes).as_bytes())
    }
}

impl std::str::FromStr for B3Hash {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() != 64 {
            return Err("a BLAKE3 digest is 64 lowercase hex digits".to_owned());
        }
        let mut bytes = [0_u8; 32];
        for (at, pair) in text.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let high = pair
                .first()
                .copied()
                .ok_or_else(|| "short digest".to_owned())?;
            let low = pair
                .get(1)
                .copied()
                .ok_or_else(|| "short digest".to_owned())?;
            let value = (hex_digit(high)? << 4) | hex_digit(low)?;
            let slot = bytes
                .get_mut(at)
                .ok_or_else(|| "a BLAKE3 digest is 32 bytes".to_owned())?;
            *slot = value;
        }
        Ok(B3Hash(bytes))
    }
}

fn hex_digit(byte: u8) -> Result<u8, String> {
    let invalid = || "a BLAKE3 digest is 64 lowercase hex digits".to_owned();
    match byte {
        b'0'..=b'9' => byte.checked_sub(b'0').ok_or_else(invalid),
        b'a'..=b'f' => byte
            .checked_sub(b'a')
            .and_then(|value| value.checked_add(10))
            .ok_or_else(invalid),
        _ => Err(invalid()),
    }
}

impl std::fmt::Display for B3Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_hex(f, &self.0)
    }
}

impl Serialize for B3Hash {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        write_digest(self, &self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for B3Hash {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        read_digest(deserializer).map(B3Hash)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GitOid([u8; 20]);

impl GitOid {
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        GitOid(bytes)
    }

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        decode_hex_fixed::<20>(raw).map(GitOid)
    }
}

impl std::fmt::Display for GitOid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_hex(f, &self.0)
    }
}

impl Serialize for GitOid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        write_digest(self, &self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for GitOid {

    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        read_digest(deserializer).map(GitOid)
    }
}

fn write_digest<const N: usize, S: serde::Serializer>(
    spelled: &impl std::fmt::Display,
    bytes: &[u8; N],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    [u8; N]: Serialize,
{
    if serializer.is_human_readable() {
        serializer.collect_str(spelled)
    } else {
        bytes.serialize(serializer)
    }
}

fn read_digest<'de, const N: usize, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<[u8; N], D::Error>
where
    [u8; N]: Deserialize<'de>,
{
    if !deserializer.is_human_readable() {
        return <[u8; N]>::deserialize(deserializer);
    }
    let raw = String::deserialize(deserializer)?;
    decode_hex_fixed::<N>(&raw).ok_or_else(|| {
        serde::de::Error::custom(format_args!(
            "expected exactly {} lowercase hex digits",
            N.saturating_mul(2)
        ))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    Lines { from: u64, to: u64 },
    Bytes { from: u64, to: u64 },
}

impl Range {

    pub fn lines(from: u64, to: u64) -> Result<Self, AxError> {
        if from == 0 || from > to {
            return Err(range_error("lines", from, to));
        }
        Ok(Range::Lines { from, to })
    }

    pub fn bytes(from: u64, to: u64) -> Result<Self, AxError> {
        if from > to {
            return Err(range_error("bytes", from, to));
        }
        Ok(Range::Bytes { from, to })
    }
}

fn range_error(unit: &str, from: u64, to: u64) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "construct range",
        format!("{unit} {from}-{to}"),
    )
    .with_recovery("require from <= to; line ranges start at 1, byte ranges at 0")
}

impl std::fmt::Display for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Range::Lines { from, to } => write!(f, "L{from}-{to}"),
            Range::Bytes { from, to } => write!(f, "B{from}-{to}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Locator {
    Cas {
        hash: B3Hash,
        range: Option<Range>,
    },
    File {
        address: Address,
        oid: GitOid,
        range: Option<Range>,
    },
}

impl Locator {

    #[must_use]
    pub const fn cas(hash: B3Hash) -> Self {
        Locator::Cas { hash, range: None }
    }

    pub fn parse(raw: &str) -> Result<Self, AxError> {
        let parsed = Locator::parse_inner(raw)?;

        let echo = parsed.to_string();
        if echo != raw {
            return Err(invalid(
                raw,
                format!("non-canonical spelling; use `{echo}`"),
            ));
        }
        Ok(parsed)
    }

    fn parse_inner(raw: &str) -> Result<Self, AxError> {
        if let Some(rest) = raw.strip_prefix("cas:") {
            let (body, range) = split_range(raw, rest)?;
            let hex = body.strip_prefix("b3-").ok_or_else(|| {
                invalid(
                    raw,
                    "unknown or missing algorithm tag; the only tag is `b3-`",
                )
            })?;
            let hash = decode_hex_fixed::<32>(hex)
                .map(B3Hash)
                .ok_or_else(|| invalid(raw, "hash must be exactly 64 lowercase hex digits"))?;
            return Ok(Locator::Cas { hash, range });
        }
        if let Some(rest) = raw.strip_prefix("file:") {
            let (body, range) = split_range(raw, rest)?;
            let (addr_raw, oid_raw) = body
                .rsplit_once('@')
                .ok_or_else(|| invalid(raw, "file locator needs `<address>@<git-oid>`"))?;
            let address = Address::parse(addr_raw)
                .map_err(|_| invalid(raw, "address part violates the address grammar"))?;
            let oid = GitOid::parse(oid_raw)
                .ok_or_else(|| invalid(raw, "git oid must be exactly 40 lowercase hex digits"))?;
            return Ok(Locator::File {
                address,
                oid,
                range,
            });
        }
        if raw.starts_with("secret:") {
            return Err(invalid(
                raw,
                "SecretRef never enters the locator grammar (separate parser by design)",
            ));
        }
        Err(invalid(
            raw,
            "unknown scheme; only `cas:` and `file:` exist",
        ))
    }
}

impl std::fmt::Display for Locator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Locator::Cas { hash, range } => {
                write!(f, "cas:b3-{hash}")?;
                write_range(f, range)
            }
            Locator::File {
                address,
                oid,
                range,
            } => {
                write!(f, "file:{address}@{oid}")?;
                write_range(f, range)
            }
        }
    }
}

impl Serialize for Locator {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Locator {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Locator::parse(&raw).map_err(serde::de::Error::custom)
    }
}

fn invalid(raw: &str, violation: impl Into<String>) -> AxError {
    AxError::failure(AxCode::LocatorInvalid, "parse locator", raw).with_recovery(violation.into())
}

fn split_range(raw: &str, rest: &str) -> Result<(String, Option<Range>), AxError> {
    match rest.split_once('#') {
        None => Ok((rest.to_owned(), None)),
        Some((body, fragment)) => {
            let range = parse_range(raw, fragment)?;
            Ok((body.to_owned(), Some(range)))
        }
    }
}

fn parse_range(raw: &str, fragment: &str) -> Result<Range, AxError> {
    let mut chars = fragment.chars();
    let unit = chars
        .next()
        .ok_or_else(|| invalid(raw, "empty range fragment"))?;
    let body: &str = chars.as_str();
    let (from_raw, to_raw) = body
        .split_once('-')
        .ok_or_else(|| invalid(raw, "range needs `<from>-<to>`"))?;
    let from = parse_canonical_u64(raw, from_raw)?;
    let to = parse_canonical_u64(raw, to_raw)?;
    let range = match unit {
        'L' => Range::lines(from, to),
        'B' => Range::bytes(from, to),
        _ => {
            return Err(invalid(
                raw,
                "range unit must be `L` (lines) or `B` (bytes)",
            ));
        }
    };
    range.map_err(|_| invalid(raw, "range bounds are out of order or out of base"))
}

fn parse_canonical_u64(raw: &str, digits: &str) -> Result<u64, AxError> {
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(invalid(raw, "range bound must be decimal digits"));
    }
    if digits.len() > 1 && digits.starts_with('0') {
        return Err(invalid(raw, "range bound must not carry leading zeros"));
    }
    digits
        .parse::<u64>()
        .map_err(|_| invalid(raw, "range bound exceeds u64"))
}

pub(crate) fn write_hex(f: &mut std::fmt::Formatter<'_>, bytes: &[u8]) -> std::fmt::Result {
    for b in bytes {
        write!(f, "{b:02x}")?;
    }
    Ok(())
}

fn write_range(f: &mut std::fmt::Formatter<'_>, range: &Option<Range>) -> std::fmt::Result {
    match range {
        Some(r) => write!(f, "#{r}"),
        None => Ok(()),
    }
}

pub(crate) fn decode_hex_fixed<const N: usize>(raw: &str) -> Option<[u8; N]> {
    if raw.len() != N.checked_mul(2)? {
        return None;
    }
    let mut out = Vec::with_capacity(N);
    let mut bytes = raw.bytes();
    while let Some(hi) = bytes.next() {
        let lo = bytes.next()?;
        let value = nibble(hi)?.checked_mul(16)?.checked_add(nibble(lo)?)?;
        out.push(value);
    }
    <[u8; N]>::try_from(out.as_slice()).ok()
}

fn nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => c.checked_sub(b'0'),
        b'a'..=b'f' => c.checked_sub(b'a')?.checked_add(10),
        _ => None,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests;
