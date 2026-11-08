// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Complete read intervals confirmed into a run's window, and the
//! decision to answer a repeated read with an unchanged stub.
//!
//! This is not `ReadStamps`: that table admits edits, while this one
//! remembers what the model actually received. Claims stay out of tool
//! payloads and the ledger. A speculative result is staged, admitted in
//! call order, and confirmed only if exchange compaction left it whole.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, PoisonError};

use kernel::{Address, B3Hash};

use super::stamp::Stamp;

const SHOWN_MAX_PATHS: usize = 64;
const SHOWN_MAX_PENDING: usize = 256;
const SHOWN_MAX_WINDOWS: usize = 8;

/// A requested or delivered line interval, counted from zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub offset: u64,
    pub lines: u64,
}

/// The interval an unchanged reply names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Answer {
    pub lines: u64,
    pub total_lines: u64,
}

/// What a repeated read may answer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    not(any(windows, target_os = "linux", target_os = "macos", test)),
    expect(
        dead_code,
        reason = "unchanged evidence is queried only by handle fast paths"
    )
)]
pub enum Recall {
    Fresh,
    Repeat(Answer),
    Compare { digest: B3Hash, answer: Answer },
}

/// The observations every confirmed claim carries.
pub(crate) struct Observation {
    total_lines: u64,
    bytes: usize,
    digest: Option<B3Hash>,
}

impl Observation {
    pub(crate) fn new(total_lines: u64, bytes: usize, digest: Option<B3Hash>) -> Observation {
        Observation {
            total_lines,
            bytes,
            digest,
        }
    }
}

/// Evidence produced by one successful read. Kept private to the run,
/// never serialized as a tool result.
pub(crate) struct Claim {
    addr: Address,
    stamp: Stamp,
    whole: bool,
    spans: Vec<Span>,
    #[cfg_attr(
        not(any(windows, target_os = "linux", target_os = "macos", test)),
        expect(
            dead_code,
            reason = "unchanged evidence is queried only by handle fast paths"
        )
    )]
    total_lines: u64,
    bytes: usize,
    digest: Option<B3Hash>,
    generation: u64,
}

impl Claim {
    pub(crate) fn whole(addr: Address, stamp: Stamp, observed: Observation) -> Claim {
        Claim {
            addr,
            stamp,
            whole: true,
            spans: Vec::new(),
            total_lines: observed.total_lines,
            bytes: observed.bytes,
            digest: observed.digest,
            generation: 0,
        }
    }

    pub(crate) fn window(addr: Address, stamp: Stamp, span: Span, observed: Observation) -> Claim {
        Claim {
            addr,
            stamp,
            whole: false,
            spans: vec![span],
            total_lines: observed.total_lines,
            bytes: observed.bytes,
            digest: observed.digest,
            generation: 0,
        }
    }

    fn with_generation(mut self, generation: u64) -> Claim {
        self.generation = generation;
        self
    }
}

/// The complete intervals one run has confirmed, plus the current
/// wave's speculative claims.
#[derive(Clone, Default)]
pub struct ReadShown {
    inner: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    confirmed: BTreeMap<Address, Claim>,
    staged: BTreeMap<String, Claim>,
    pending: BTreeMap<String, Claim>,
    generation: u64,
}

impl State {
    fn next_generation(&mut self) -> u64 {
        self.generation = self.generation.saturating_add(1);
        self.generation
    }
}

impl ReadShown {
    /// Answers from confirmed evidence only. A matching stamp and a
    /// covered interval are necessary but not sufficient: a racy claim
    /// must still be compared by content.
    #[cfg_attr(
        not(any(windows, target_os = "linux", target_os = "macos", test)),
        expect(
            dead_code,
            reason = "unchanged evidence is queried only by handle fast paths"
        )
    )]
    pub(crate) fn recall(&self, addr: &Address, now: &Stamp, asked: &str, want: Span) -> Recall {
        let state = self.table();
        let Some(claim) = state.confirmed.get(addr) else {
            return Recall::Fresh;
        };
        if claim.stamp != *now {
            return Recall::Fresh;
        }
        let lines = if want.offset >= claim.total_lines {
            0
        } else {
            want.lines
                .min(claim.total_lines.saturating_sub(want.offset))
        };
        let end = want.offset.saturating_add(lines);
        let covered = claim.whole
            || lines == 0
            || claim.spans.iter().any(|span| {
                span.offset <= want.offset && end <= span.offset.saturating_add(span.lines)
            });
        if !covered {
            return Recall::Fresh;
        }
        let answer = Answer {
            lines,
            total_lines: claim.total_lines,
        };
        let Some(stub) = crate::rendering::unchanged(crate::rendering::Stub {
            path: asked,
            offset: want.offset,
            lines: answer.lines,
            total_lines: answer.total_lines,
            numbering: crate::rendering::Numbering::FromOne,
        }) else {
            return Recall::Fresh;
        };
        if claim.bytes < stub.len().saturating_mul(2) {
            return Recall::Fresh;
        }
        match claim.digest {
            Some(digest) => Recall::Compare { digest, answer },
            None => Recall::Repeat(answer),
        }
    }

    /// Stages a claim after a successful read, before admission.
    pub(crate) fn stage(&self, call: &str, claim: Claim) {
        let mut state = self.table();
        let generation = state.next_generation();
        state
            .staged
            .insert(call.to_owned(), claim.with_generation(generation));
        trim(&mut state.staged, SHOWN_MAX_PENDING);
    }

    /// Discards evidence from an attempt whose result was not delivered.
    pub fn discard(&self, call: &str) {
        self.table().staged.remove(call);
    }

    /// Moves one staged claim to pending in call order.
    pub fn admit(&self, call: &str) {
        let mut state = self.table();
        if let Some(claim) = state.staged.remove(call) {
            let generation = state.next_generation();
            state
                .pending
                .insert(call.to_owned(), claim.with_generation(generation));
            trim(&mut state.pending, SHOWN_MAX_PENDING);
        }
    }

    /// Confirms unshortened pending claims and discards speculation.
    pub fn confirm(&self, shortened: &BTreeSet<String>) {
        let mut state = self.table();
        state.staged.clear();
        let mut pending: Vec<_> = std::mem::take(&mut state.pending).into_iter().collect();
        pending.sort_by_key(|(_, claim)| claim.generation);
        for (call, claim) in pending {
            if shortened.contains(&call) {
                continue;
            }
            let generation = state.next_generation();
            let claim = claim.with_generation(generation);
            if let Some(old) = state.confirmed.get_mut(&claim.addr) {
                let compatible_digest = match (old.digest, claim.digest) {
                    (Some(before), Some(after)) => before == after,
                    _ => true,
                };
                if old.stamp == claim.stamp && compatible_digest {
                    old.whole |= claim.whole;
                    if old.whole {
                        old.spans.clear();
                    } else {
                        old.spans.extend(claim.spans);
                        let first = old.spans.len().saturating_sub(SHOWN_MAX_WINDOWS);
                        old.spans = old.spans.split_off(first);
                    }
                    old.bytes = claim.bytes;
                    old.digest = old.digest.or(claim.digest);
                    old.generation = generation;
                    continue;
                }
            }
            state.confirmed.insert(claim.addr.clone(), claim);
            trim(&mut state.confirmed, SHOWN_MAX_PATHS);
        }
    }

    fn table(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Drops the least recently touched claim once a bounded table is full.
fn trim<K: Ord + Clone>(claims: &mut BTreeMap<K, Claim>, cap: usize) {
    if claims.len() <= cap {
        return;
    }
    if let Some(oldest) = claims
        .iter()
        .min_by_key(|(_, claim)| claim.generation)
        .map(|(key, _)| key.clone())
    {
        claims.remove(&oldest);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test fixtures")]
mod tests;
