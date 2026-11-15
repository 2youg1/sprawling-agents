// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub mod clock;
pub mod console;
pub mod identity;

pub use identity::NAME;

pub use console_ffi::part;
pub use console_ffi::scene;
pub use console_ffi::{Frame, Refused, draw};
