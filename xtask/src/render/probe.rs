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

use super::engine::BUDGET_MS;
use super::pass::Pass;

/// A region that says it is still loading, as ARIA spells it.
///
/// The client marks a view it fetches as a chunk of its own this way
/// until the chunk lands. A file-served chunk does not hold the engine's
/// virtual time, so without this wait the probe raced the chunk and
/// measured a page with no first heading.
const BUSY: &str = r#"[aria-busy="true"]"#;

/// How often the probe asks again whether the page is still busy.
const POLL_MS: u32 = 100;

/// Where the walk writes what it read.
pub(super) const SINK: &str = "sprawling-render";

/// Where it writes the conditions the page drew in.
pub(super) const CONDITIONS: &str = "sprawling-render-conditions";

/// Where it writes the words the page declares.
pub(super) const DECLARED: &str = "sprawling-render-declared";

/// Where the page reports, if it does, that it threw.
pub(super) const FAILED: &str = "sprawling-render-failed";

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
<pre id="{FAILED}"></pre>
<script>
// **A page that threw is told apart from a page that drew nothing.**
// Both leave the sink empty, and the two need different repairs: one is
// a client defect, the other a route that renders no fixtures. Without
// this the gate said "drew nothing measurable" for a `ParseError` at
// import time, and finding that cost an hour of bisecting a bundle.
window.addEventListener('error', function (e) {{
  var thrown = e.error || {{}};
  var where = (e.filename || '') + ':' + (e.lineno || '') + ':' + (e.colno || '');
  document.getElementById('{FAILED}').textContent =
    (e.message || 'uncaught') + ' at ' + where + (thrown.stack ? ' :: ' + thrown.stack : '');
}});
window.addEventListener('unhandledrejection', function (e) {{
  var reason = e.reason || {{}};
  document.getElementById('{FAILED}').textContent =
    'unhandled rejection :: ' + (reason.stack || reason.message || String(reason));
}});
</script>
<script>
// Measure once nothing reads busy, leaving two polls of the budget for
// the dump; a page still busy then is reported as one that never
// settled rather than judged as the half-drawn page it is.
var waited = {SETTLE_MS};
function measure() {{
  if (document.querySelector('{BUSY}')) {{
    if (waited + {poll_twice} < {BUDGET_MS}) {{
      waited += {POLL_MS};
      setTimeout(measure, {POLL_MS});
      return;
    }}
    document.getElementById('{FAILED}').textContent =
      'a region still read aria-busy after ' + waited + ' virtual ms';
    return;
  }}
  var read = (function () {{{}}})();
  document.getElementById('{SINK}').textContent = read.sink;
  document.getElementById('{DECLARED}').textContent = read.declared;
  document.getElementById('{CONDITIONS}').textContent = read.conditions;
}}
setTimeout(measure, {SETTLE_MS});
</script>
"#,
        body(Some(pass.theme())),
        poll_twice = POLL_MS.saturating_mul(2),
    )
}

#[cfg(test)]
mod tests {
    use super::super::pass::wanted;
    use super::script;
    use crate::report::XtaskError;

    /// A view the client fetches as a chunk of its own lands after the
    /// page's first paint, and a file-served chunk does not hold the
    /// engine's virtual time. The probe has to wait for the region that
    /// says it is still loading, or it measures a page with no heading.
    #[test]
    fn the_probe_waits_while_a_region_reads_busy() -> Result<(), XtaskError> {
        let probe = wanted()?.into_iter().map(script).collect::<String>();
        assert!(
            probe.contains(r#"querySelector('[aria-busy="true"]')"#),
            "the probe measures without asking whether a region is still loading"
        );
        Ok(())
    }
}
