// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one reading of an element's box.
//!
//! Three callers measure boxes: the `measure` action reports them, a
//! `refs` shot unions them into the region it covers, and `survey` judges
//! a page by them. All three splice the same function into the script
//! they send, so the four numbers are rounded once and two of them cannot
//! come back a pixel apart.

/// One element's box, as four whole CSS pixels.
///
/// Whole pixels rather than the sub-pixel rectangle the engine lays out:
/// every box this crate carries sits in a ledger payload beside integers,
/// and a fraction rounded twice is how two readings of one element start
/// disagreeing. `left`/`top` rather than `x`/`y` because both are the same
/// number and only the first is spelled in every engine this crate drives.
pub(crate) const BOX_OF: &str = "function boxOf(node) {\n  \
    var rect = node.getBoundingClientRect();\n  \
    return [Math.round(rect.left), Math.round(rect.top), Math.round(rect.width), \
    Math.round(rect.height)];\n\
    }";
