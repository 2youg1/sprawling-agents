// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod command;
mod environment;

#[cfg(windows)]
pub use command::NO_WINDOW;
pub use command::command;
pub use environment::{PAIRING_TOKEN, SECRET_PREFIX};
