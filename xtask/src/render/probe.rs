// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The script that measures the page, the three elements it writes its
//! readings into, and the reader of every sentence it writes
//! (xtask-SPEC.md section 8-17).
//!
//! **The writer and the reader of one sentence live in one file.** The
//! probe's record is a line of positional fields with no schema behind
//! it, so a field inserted in the script and not in the parser silently
//! shifts every field after it and the gate goes on reporting numbers
//! that are no longer what they are named. Putting the two within a
//! screen of one another is the whole of the defence available here.

use browser::survey::probe::{SETTLE_MS, body};

pub(super) use browser::survey::probe::declared;

use super::pass::Pass;

/// Where the walk writes what it read.
pub(super) const SINK: &str = "sprawling-render";

/// Where it writes the conditions the page drew in.
pub(super) const CONDITIONS: &str = "sprawling-render-conditions";

/// Where it writes the words the page declares.
pub(super) const DECLARED: &str = "sprawling-render-declared";

/// The measurement, wrapped so a dumped DOM carries it back.
///
/// The engine this gate drives renders once and prints the document, so
/// the three strings have to be somewhere in that document. They are
/// three `<pre>` elements this script appends, read back out by
/// `engine`. A resident surveying a live page takes the same three
/// strings as the value of one evaluation and appends nothing
/// ([`browser::survey::probe::evaluated`]); the measurement between the
/// two is one function.
pub(super) fn script(pass: &Pass) -> String {
    format!(
        r#"<pre id="{SINK}"></pre>
<pre id="{CONDITIONS}"></pre>
<pre id="{DECLARED}"></pre>
<script>
setTimeout(function () {{
  var read = (function () {{{}}})();
  document.getElementById('{SINK}').textContent = read.sink;
  document.getElementById('{DECLARED}').textContent = read.declared;
  document.getElementById('{CONDITIONS}').textContent = read.conditions;
}}, {SETTLE_MS});
</script>
"#,
        body(Some(pass.theme()))
    )
}
