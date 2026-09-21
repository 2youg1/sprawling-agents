// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Driving a browser: the seam, the conversation, what a model may see
//! of a page, what it may do to one, the loop around a change, and where
//! the browser keeps what it remembers.
//!
//! The protocol layer is pure. Frames are built and replies are read
//! without a socket in sight, so a whole session is asserted without a
//! browser and the binary owns the one place bytes actually move.

mod act;
mod devloop;
mod diff;
mod input;
mod port;
mod profile;
mod session;
mod shot;
mod snapshot;
pub mod survey;
mod verb;

pub use act::{Action, Origin, Point, STEPS_MAX, frame_for, resolve_frame};
pub use devloop::{DevLoop, LOOKS_MAX, Observation, QUIET_LOOKS, Step};
pub use diff::{Box2, Difference, diff};
pub use input::{Origin as ResolvedOrigin, shared_id_of};
pub use port::{BrowserPort, Frame, Reply};
pub use profile::{PROFILES_DIR, Profile};
pub use session::{ContextId, Recording, Session, SessionRequest};
pub use shot::{Clip, Shot, ShotRequest};
pub use snapshot::{Node, PageSnapshot};
pub use verb::{Verb, complained, read_json};
