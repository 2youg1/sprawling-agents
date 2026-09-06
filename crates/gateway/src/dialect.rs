// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which dialect answers a question, and nothing about how it answers.
//!
//! Five entrances, one `match` each, and a closed set of two: a dialect
//! this build cannot translate is refused rather than approximated with
//! the nearer of the two it knows. What each dialect does with a request
//! is `gateway::anthropic`'s and `gateway::openai`'s; what they share is
//! `gateway::mismatch`'s.
//!
//! The canonical conversation is Anthropic-shaped, so the losses are all
//! on the other path and each is documented where it is taken.
//!
//! No I/O, no state, no clock: byte-for-byte explainable requests are
//! the whole point of writing the wire format ourselves.

mod request;
mod response;

pub use request::request_wire;
pub use response::{increment_of, response_from_wire, response_wire, settled_from_stream};
