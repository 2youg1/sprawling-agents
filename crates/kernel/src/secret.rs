// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Secret shape judgment, the `secret:` reference grammar and `Sealed<T>`
//!. Detection is shape-table-first, entropy-second;
//! both are hand-written, regex-free, backtracking-free and float-free —
//! entropy runs in fixed point so the judgment is deterministic and
//! kani-provable. False positives are the accepted normal: the entrance
//! replaces losslessly, only the exits refuse.

mod scan;
mod sealed;
mod span;

pub use scan::scan;
pub use sealed::Sealed;
pub use span::{SecretRef, SecretSpan};
