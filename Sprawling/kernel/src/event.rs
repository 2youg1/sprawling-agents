// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod identity;
mod kind;
mod moment;
mod payload;
pub mod record;
mod scope;
mod who;

pub use identity::{RunId, Seq, TimeMs};
pub use kind::{EventKind, WindowClass};
pub use payload::{EventDraft, EventRecord, EventRef, Payload};
pub use scope::Scope;
pub use who::{Who, escape_markup};
