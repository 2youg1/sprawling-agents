// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which machine this build is standing on.
//!
//! Two files, chosen by `cfg`, and no trait between them: a trait with
//! one implementation per build is decoration (ARCHITECTURE.md section
//! 4), and the two arms are never both present to be swapped.
//!
//! Both arms present the same one type, [`Desk`], and a desk belongs to
//! one connection. That is not cosmetic: two things live between
//! calls — which generation a
//! window's refs belong to, and which recordings are running — and
//! process-wide state would let a second connection, under a second
//! scope file, stop the first one's recording.

#[cfg(not(windows))]
mod elsewhere;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
pub(crate) use elsewhere::Desk;
#[cfg(windows)]
pub(crate) use windows::Desk;
