// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What kind of provider failure an `E_PROVIDER` error reports, and
//! whether the account that met a failure can still take the request.
//! The part `crates/kernel/spec/Error.lean` specifies this module with the
//! rest of `kernel::error`.

use serde::{Deserialize, Serialize};

/// The kind of one provider failure, as the call site that saw it named
/// it. It travels on the wire so a page can say it in the reader's own
/// language; the city's recovery sentence stays beside it for the fold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ProviderFailureKind {
    /// The exchange never completed: the request did not reach the
    /// provider, the answer stopped before its end, or the transport's
    /// deadline passed.
    Exchange,
    /// A streamed body could not be read further off the socket.
    Cut,
    /// A stream stayed open and silent past the bound set for silence.
    Silence,
    /// The provider answered, and its status refuses the request. Held
    /// as `u32` because the wire's schema subset has no upper bound to
    /// spell a `u16` with.
    Refused { status: u32 },
    /// The provider refused the request because this account's quota is
    /// used up (gateway D32). Asked again it answers the same; another
    /// account, or credit added to this one, is the way out.
    Quota { status: u32 },
    /// The provider refused the request because it no longer fits the
    /// model's context window.
    Overflow { status: u32 },
    /// The provider answered 2xx with a body this city cannot read.
    Unreadable,
    /// A stream that already answered 2xx carried an error frame. The
    /// provider's type for it stays in the error's subject.
    Reported,
    /// The request could not be built on this side.
    Unbuilt,
}

/// Whether the account a request went out on can still take it
/// (`crates/kernel/spec/Error.lean` D53).
///
/// A second answer beside `Retry` rather than a reading of it:
/// a rejected key and a request the provider calls malformed are both
/// `Retry::No`, and only the first is mended by another account. On the
/// wire `"keep"` or `"advance"`; `Keep` is left out, so a record written
/// before this field existed reads as `Keep` and keeps its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AccountDisposition {
    /// This failure says nothing about the account: whether the request
    /// goes out again is `retry`'s answer alone.
    #[default]
    Keep,
    /// This account cannot take the request - its key was rejected, its
    /// quota is used up, or its credential is missing - and the next
    /// account may.
    Advance,
}

impl AccountDisposition {
    /// The serde skip test: `Keep` is the absence of the field.
    pub(crate) fn is_keep(&self) -> bool {
        *self == AccountDisposition::Keep
    }
}
