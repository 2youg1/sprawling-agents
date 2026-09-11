// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Turning a recording of speech into a line of text, through an
//! endpoint speaking the OpenAI audio wire format.
//!
//! **This endpoint is optional.** A city with none attached does
//! everything else it does; what it cannot do, it says by name, with a
//! code and a recovery, at the moment it is asked.

mod chosen;
mod recording;
mod transcriber;
mod wire;

pub use chosen::transcriber_for;
pub use recording::{AudioType, Recording};
pub use transcriber::{Transcriber, TranscriberConfig};
