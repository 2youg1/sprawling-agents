// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Root Nix lock digest classification (tools/xtask/Spec.lean §8-9).
//! Round-trip equality binds a parsed property to its original bytes;
//! unfamiliar encodings admit no exemption and remain fully scanned.

use serde_json::Value;

const NODES: &str = "nodes";
const LOCKED: &str = "locked";
const NAR_HASH: &str = "narHash";

/// Original-byte spans of canonical pinned GitHub NAR digest properties.
/// Construction failures retain the ordinary scan of every byte.
pub(super) struct NarHashes(Vec<(usize, usize)>);

impl NarHashes {
    pub(super) fn classify(rel: &str, bytes: &[u8]) -> Self {
        if rel != "flake.lock" {
            return Self(Vec::new());
        }
        let Ok(lock) = serde_json::from_slice::<Value>(bytes) else {
            return Self(Vec::new());
        };
        let Ok(mut canonical) = serde_json::to_vec_pretty(&lock) else {
            return Self(Vec::new());
        };
        canonical.push(b'\n');
        if canonical != bytes || lock.get("version").and_then(Value::as_u64) != Some(7) {
            return Self(Vec::new());
        }
        let Some(nodes) = lock.get(NODES).and_then(Value::as_object) else {
            return Self(Vec::new());
        };
        let Some(root) = lock.get("root").and_then(Value::as_str) else {
            return Self(Vec::new());
        };
        if !nodes.get(root).is_some_and(Value::is_object) {
            return Self(Vec::new());
        }
        let mut spans = Vec::new();
        for (name, node) in nodes {
            let Some(locked) = node.get(LOCKED).and_then(Value::as_object) else {
                continue;
            };
            let Some(hash) = locked.get(NAR_HASH).and_then(Value::as_str) else {
                continue;
            };
            if locked.get("type").and_then(Value::as_str) != Some("github")
                || !["owner", "repo"].into_iter().all(|field| {
                    locked
                        .get(field)
                        .and_then(Value::as_str)
                        .is_some_and(|name| {
                            !name.is_empty()
                                && name.bytes().all(|byte| {
                                    byte.is_ascii_alphanumeric()
                                        || matches!(byte, b'-' | b'_' | b'.')
                                })
                        })
                })
                || !locked
                    .get("rev")
                    .and_then(Value::as_str)
                    .is_some_and(|rev| {
                        rev.len() == 40
                            && rev
                                .bytes()
                                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                    })
                || !is_sha256_sri(hash)
            {
                continue;
            }
            let mut changed = lock.clone();
            let Some(value) = changed
                .get_mut(NODES)
                .and_then(|nodes| nodes.get_mut(name))
                .and_then(|node| node.get_mut(LOCKED))
                .and_then(|locked| locked.get_mut(NAR_HASH))
            else {
                return Self(Vec::new());
            };
            *value = Value::String(String::new());
            let Ok(changed) = serde_json::to_vec_pretty(&changed) else {
                return Self(Vec::new());
            };
            let start = bytes
                .iter()
                .zip(&changed)
                .take_while(|(a, b)| a == b)
                .count();
            let end = start.saturating_add(hash.len());
            if bytes.get(start..end) != Some(hash.as_bytes()) {
                return Self(Vec::new());
            }
            spans.push((start, hash.len().saturating_sub(1)));
        }
        Self(spans)
    }

    /// Provider matches always remain findings, including inside an admitted digest.
    pub(super) fn admits(&self, span: &kernel::secret::SecretSpan) -> bool {
        span.provider.is_none() && self.0.contains(&(span.start, span.len))
    }
}

fn is_sha256_sri(hash: &str) -> bool {
    hash.strip_prefix("sha256-")
        .and_then(|digest| digest.strip_suffix('='))
        .is_some_and(|digest| {
            digest.len() == 43
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/'))
                && digest
                    .as_bytes()
                    .last()
                    .is_some_and(|byte| b"AEIMQUYcgkosw048".contains(byte))
        })
}
