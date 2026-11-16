// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use browser::survey::probe::{SETTLE_MS, body};

pub(super) use browser::survey::probe::declared;

use super::engine::BUDGET_MS;
use super::pass::Pass;

pub(crate) const PENDING: &str = "[data-pending]";

pub(crate) const POLL_MS: u32 = 100;

pub(super) const SINK: &str = "sprawling-render";

pub(super) const CONDITIONS: &str = "sprawling-render-conditions";

pub(super) const DECLARED: &str = "sprawling-render-declared";

pub(super) const FAILED: &str = "sprawling-render-failed";

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
// Measure once no view is pending, leaving two polls of the budget for
// the dump; a page still pending then is reported as one that never
// settled rather than judged as the half-drawn page it is.
var waited = {SETTLE_MS};
function measure() {{
  if (document.querySelector('{PENDING}')) {{
    if (waited + {poll_twice} < {BUDGET_MS}) {{
      waited += {POLL_MS};
      setTimeout(measure, {POLL_MS});
      return;
    }}
    document.getElementById('{FAILED}').textContent =
      'a lazily loaded view was still pending after ' + waited + ' virtual ms';
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

    #[test]
    fn the_probe_waits_while_a_view_is_pending() -> Result<(), XtaskError> {
        let probe = wanted()?.into_iter().map(script).collect::<String>();
        assert!(
            probe.contains("querySelector('[data-pending]')"),
            "the probe measures without asking whether a region is still loading"
        );
        Ok(())
    }
}
