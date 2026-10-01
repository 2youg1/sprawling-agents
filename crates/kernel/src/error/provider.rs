// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What kind of provider failure an `E_PROVIDER` error reports.
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
