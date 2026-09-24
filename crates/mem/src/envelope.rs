// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The safe face over the kernel's envelope span scanner: locate the raw
//! value of each envelope key inside one JSON object, without allocating
//! and without decoding. The bytes a caller gets back are borrowed from
//! the input for as long as the input lives - the borrow shape the raw
//! value takes on the serde side (mem-SPEC.md section 8-1).

use kernel::error::{AxCode, AxError};

/// The five names the envelope probe reads. This enum and the kernel's
/// key table are two spellings of one fact, held together by the proptest
/// equivalence suite (mem-SPEC.md section 12, decision 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeKey {
    V,
    Seq,
    Prev,
    Kind,
    Ig,
}

impl EnvelopeKey {
    /// Every key, in slot order.
    pub const ALL: [EnvelopeKey; 5] = [
        EnvelopeKey::V,
        EnvelopeKey::Seq,
        EnvelopeKey::Prev,
        EnvelopeKey::Kind,
        EnvelopeKey::Ig,
    ];

    /// The spelling a line's bytes carry.
    pub fn name(self) -> &'static str {
        match self {
            EnvelopeKey::V => "v",
            EnvelopeKey::Seq => "seq",
            EnvelopeKey::Prev => "prev",
            EnvelopeKey::Kind => "kind",
            EnvelopeKey::Ig => "ig",
        }
    }

    /// The slot this key lands in, and its bit in `found`.
    fn slot(self) -> usize {
        match self {
            EnvelopeKey::V => 0,
            EnvelopeKey::Seq => 1,
            EnvelopeKey::Prev => 2,
            EnvelopeKey::Kind => 3,
            EnvelopeKey::Ig => 4,
        }
    }

    fn bit(self) -> u32 {
        match self {
            EnvelopeKey::V => 1,
            EnvelopeKey::Seq => 2,
            EnvelopeKey::Prev => 4,
            EnvelopeKey::Kind => 8,
            EnvelopeKey::Ig => 16,
        }
    }
}

/// One located value, `[start, start + len)` in the caller's bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Span {
    start: usize,
    len: usize,
}

impl Span {
    fn slice(self, input: &[u8]) -> Option<&[u8]> {
        let end = self.start.checked_add(self.len)?;
        input.get(self.start..end)
    }
}

/// What every scan answers with: the input it borrowed from, and one
/// presence-flagged span per key.
pub struct EnvelopeSpans<'input> {
    input: &'input [u8],
    slots: [Option<Span>; 5],
}

impl<'input> EnvelopeSpans<'input> {
    /// The key's value as the bytes carry it: raw JSON text, strings
    /// still quoted and escapes undecoded. `None` when the key is absent
    /// from the object.
    pub fn get(&self, key: EnvelopeKey) -> Option<&'input [u8]> {
        let slot = self.slots.get(key.slot()).copied().flatten()?;
        slot.slice(self.input)
    }
}

/// The kernel's answers, in this crate's vocabulary. The integer codes
/// have one home (`zig/src/lib.zig`, `Code`) and one mapping point
/// ([`KernelRefusal::into_error`]); a code that mapping does not name is
/// refused rather than read as anything (mem-SPEC.md section 8-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KernelRefusal {
    Malformed,
    DuplicateName,
    TooDeep,
    UnknownCode(i32),
    SpanOutsideInput,
}

const CODE_OK: i32 = 0;
const CODE_MALFORMED: i32 = -1;
const CODE_DUPLICATE_NAME: i32 = -2;
const CODE_TOO_DEEP: i32 = -3;

fn classify(code: i32) -> Result<(), KernelRefusal> {
    match code {
        CODE_OK => Ok(()),
        CODE_MALFORMED => Err(KernelRefusal::Malformed),
        CODE_DUPLICATE_NAME => Err(KernelRefusal::DuplicateName),
        CODE_TOO_DEEP => Err(KernelRefusal::TooDeep),
        other => Err(KernelRefusal::UnknownCode(other)),
    }
}

impl KernelRefusal {
    /// The single place a kernel answer becomes an `AxError`, carrying
    /// the action, the subject and a recovery the caller can act on.
    fn into_error(self) -> AxError {
        match self {
            KernelRefusal::Malformed => {
                AxError::failure(AxCode::InvalidArgs, "scan the envelope", "one JSON object")
                    .with_recovery(
                        "send exactly one JSON object and nothing after it, then scan again",
                    )
            }
            KernelRefusal::DuplicateName => AxError::failure(
                AxCode::InvalidArgs,
                "scan the envelope",
                "a key name spelled twice",
            )
            .with_recovery(
                "keep each envelope key once: a line with two spellings of one key is not \
                         one this city writes",
            ),
            KernelRefusal::TooDeep => AxError::failure(
                AxCode::InvalidArgs,
                "scan the envelope",
                "a value nested past the kernel's depth limit",
            )
            .with_recovery("flatten the value, or store its body in the CAS and scan the locator"),
            KernelRefusal::UnknownCode(code) => AxError::failure(
                AxCode::WireMismatch,
                "map the kernel's answer",
                format!("code {code}"),
            )
            .with_recovery(
                "build zig/ and crates/mem from the same tree: a code this adapter does not know \
                 is refused rather than guessed",
            ),
            KernelRefusal::SpanOutsideInput => AxError::failure(
                AxCode::WireMismatch,
                "read a scanned span",
                "a span outside the input",
            )
            .with_recovery(
                "build zig/ and crates/mem from the same tree: the kernel may only name spans \
                 inside the bytes it was given",
            ),
        }
    }
}

/// The kernel's flat result. Field order is the C ABI's and must match
/// `Spans` in `zig/src/lib.zig`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct RawSpan {
    start: usize,
    len: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct RawSpans {
    found: u32,
    v: RawSpan,
    seq: RawSpan,
    prev: RawSpan,
    kind: RawSpan,
    ig: RawSpan,
}

#[expect(
    unsafe_code,
    reason = "the Zig kernel is reachable only through its C ABI; each declaration's contract is \
              stated beside it, and the call site states the preconditions that make it sound"
)]
unsafe extern "C" {
    /// Fills `out` with the span of each envelope key found in `input`,
    /// and answers 0 or one negative `CODE_*` value. It reads exactly
    /// `len` bytes at `input`, writes only through `out`, allocates
    /// nothing, and retains nothing once it returns.
    fn mem_envelope_spans(input: *const u8, len: usize, out: *mut RawSpans) -> i32;
}

#[expect(
    unsafe_code,
    reason = "the one call into the Zig kernel; the precondition is stated at the block"
)]
fn ask(input: &[u8]) -> (i32, RawSpans) {
    let mut raw = RawSpans::default();
    // SAFETY: `input` is a live slice, so `as_ptr()` is valid for `len`
    // bytes for the whole call (empty inputs are never dereferenced by
    // the kernel); `raw` is a local this call alone holds, so no alias
    // exists while the kernel writes it; the kernel allocates nothing and
    // keeps nothing after returning, so nothing outlives the borrow.
    let code = unsafe { mem_envelope_spans(input.as_ptr(), input.len(), &mut raw) };
    (code, raw)
}

/// Turns the flat answer into the typed one, refusing a span the kernel
/// named outside the input instead of trusting it.
fn located<'input>(
    input: &'input [u8],
    raw: &RawSpans,
) -> Result<EnvelopeSpans<'input>, KernelRefusal> {
    let mut slots: [Option<Span>; 5] = [None, None, None, None, None];
    for (slot, key) in slots.iter_mut().zip(EnvelopeKey::ALL) {
        if (raw.found & key.bit()) == 0 {
            continue;
        }
        let named = match key {
            EnvelopeKey::V => raw.v,
            EnvelopeKey::Seq => raw.seq,
            EnvelopeKey::Prev => raw.prev,
            EnvelopeKey::Kind => raw.kind,
            EnvelopeKey::Ig => raw.ig,
        };
        let span = Span {
            start: named.start,
            len: named.len,
        };
        if span.slice(input).is_none() {
            return Err(KernelRefusal::SpanOutsideInput);
        }
        *slot = Some(span);
    }
    Ok(EnvelopeSpans { input, slots })
}

/// Scans one JSON object for the envelope keys. The answer borrows from
/// `input`; nothing is copied and nothing is decoded (mem-SPEC.md
/// section 8-1).
pub fn scan(input: &[u8]) -> Result<EnvelopeSpans<'_>, AxError> {
    let (code, raw) = ask(input);
    classify(code).map_err(KernelRefusal::into_error)?;
    located(input, &raw).map_err(KernelRefusal::into_error)
}

#[cfg(test)]
mod tests;
