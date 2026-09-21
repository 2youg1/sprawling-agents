// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! OpenAI's responses face, in both directions.
//!
//! **Every shape here is read off the provider's own specification, not
//! off a client library.** The source is `openai/openai-openapi`, the
//! document `openapi.yaml` declares as API version 2.3.0, read at
//! commit `ddface9b`. A field that looks wrong is usually a field that
//! moved, so check that document before changing anything below.
//!
//! **This is a third dialect and not a variant of the second.** The
//! chat face sends `messages` and reads `choices`; this one sends
//! `input` and reads `output`, an array whose entries are messages,
//! function calls and reasoning rather than one message with fields
//! hanging off it. The stream is a third grammar again: named events
//! (`response.output_text.delta`) instead of anonymous chunks. Folding
//! the two would mean one writer branching on a flag at every step,
//! which is the shape that lets a change meant for one face reach the
//! other.
//!
//! **What this face carries that the chat face cannot:**
//!
//! - Explicit prompt-cache breakpoints. A `SystemBlock` marked `cache`
//!   becomes an `input_text` part with `prompt_cache_breakpoint`, so
//!   the segment edges this city already computes are stated rather
//!   than left to prefix matching.
//! - A settled answer at the end of its own stream. The terminal event
//!   carries the whole response object, so reassembly reads it instead
//!   of stitching fragments — and the streamed call and the blocking
//!   call reach the one parser through the same bytes.
//!
//! **The loss it takes, and takes deliberately:** a `Thinking` block
//! from an earlier turn is not sent back. This face accepts a
//! `reasoning` item only with the identifier and the encrypted content
//! the provider itself issued, and this city stores neither; a
//! reconstructed one would be refused at the first call. The canonical
//! record keeps the block, so nothing is lost from the history.

mod reply;
mod request;
mod stream;

pub(crate) use reply::{response_from, response_wire};
pub(crate) use request::request;
pub(crate) use stream::{increment_of, settled};
