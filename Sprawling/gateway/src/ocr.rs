// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading the text in a picture, through the model a person chose for
//! `ocr` (`crates/gateway/spec/Ocr.lean` §8-34).
//!
//! **This model is optional.** A city with none chosen does everything
//! else it does; what it cannot do, it says by name, with a code and a
//! recovery, at the moment it is asked.

mod chosen;
mod picture;
mod recogniser;

pub use chosen::recogniser_for;
pub use picture::Picture;
pub use recogniser::Recogniser;
