// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The script that measures the page, and the two elements it writes
//! its readings into (xtask-SPEC.md section 8-17).

use super::pass::Pass;

/// The element the probe writes its measurements into.
pub(super) const SINK: &str = "sprawling-render";

/// The element the probe writes the conditions it drew under into.
pub(super) const CONDITIONS: &str = "sprawling-render-conditions";

/// How long the probe waits for the client to mount before measuring.
///
/// Virtual time, not wall time: `--virtual-time-budget` advances timers as
/// fast as the work allows, so this is a number of frames' worth of
/// scheduling rather than a second of somebody's life.
const SETTLE_MS: u32 = 1200;

/// The measuring script.
///
/// It writes one line per element into a `<pre>` the dump then carries
/// back, because `--dump-dom` returns the document and nothing else:
/// anything the gate wants to know has to be in the document when it is
/// dumped.
///
/// The accessible name is taken the way a reader gets it — an explicit
/// `aria-label`, the element a `aria-labelledby` points at, a `title`, an
/// `alt`, the `<label>` a box is wrapped in or pointed at by, or the text
/// the element actually contains. It is not a computed accessibility tree
/// and does not claim to be; it is what the four authoring mistakes this
/// gate exists for all show up in.
///
/// Two more readings travel with each element: the centre of the first
/// painted box inside it, and whether a line is drawn under its text.
/// A decoration reaches every in-flow descendant, so the second is a
/// walk upwards that stops at the first box nothing propagates into —
/// an atomic inline, a float, or a box taken out of flow.
///
/// **The lighting is set here, on the root element, in the attribute
/// `theme.css` selects on.** The client applies the person's own choice
/// at mount out of browser storage, so a pass that did not say which
/// lighting it wanted would measure whichever page the machine running
/// the gate happens to prefer.
///
/// **What the engine does not render is not measured.** The contents of
/// a closed `<details>` still report a box, and it is a box drawn
/// nowhere near where the contents will appear when it opens: the
/// engine skips laying them out and answers `checkVisibility()` with
/// `false`, which is the reading taken here.
///
/// **A box that scrolls is measured whether or not it is one of the
/// elements named below.** Containment is judged against the box that
/// holds an element, and for anything inside a scrolling `<div>` that
/// box is the `<div>` - which, unmeasured, left every scrolling table in
/// this client judged against a container two levels further out that
/// does not scroll, two pixels away from failing a table that is right.
pub(super) fn script(pass: &Pass) -> String {
    let theme = pass.theme();
    format!(
        r#"<pre id="{SINK}"></pre>
<pre id="{CONDITIONS}"></pre>
<script>
setTimeout(function () {{
  document.documentElement.dataset.theme = '{theme}';
  var out = [];
  var seen = [];
  function named(node) {{
    var label = node.getAttribute('aria-label');
    if (label) return label;
    var by = node.getAttribute('aria-labelledby');
    if (by) {{
      var target = document.getElementById(by);
      if (target) return (target.textContent || '').trim();
    }}
    var title = node.getAttribute('title');
    if (title) return title;
    var alt = node.getAttribute('alt');
    if (alt) return alt;
    var own = (node.textContent || '').trim();
    if (own) return own;
    var wrapping = node.closest('label');
    if (wrapping) return (wrapping.textContent || '').trim();
    if (node.id) {{
      var pointed = document.querySelector('label[for="' + node.id + '"]');
      if (pointed) return (pointed.textContent || '').trim();
    }}
    return '';
  }}
  function firstMark(node) {{
    var inside = node.querySelectorAll('*');
    for (var m = 0; m < inside.length; m++) {{
      var mark = inside[m].getBoundingClientRect();
      if (mark.width > 0 && mark.height > 0) return Math.round(mark.left + mark.width / 2);
    }}
    return -1;
  }}
  function atomic(style) {{
    return style.display.indexOf('inline-') === 0 || style.display === 'inline-block'
      || style.position === 'absolute' || style.position === 'fixed' || style.cssFloat !== 'none';
  }}
  function underlined(node) {{
    for (var up = node; up; up = up.parentElement) {{
      var style = getComputedStyle(up);
      if ((style.textDecorationLine || '').indexOf('underline') >= 0) return 1;
      if (up !== node && atomic(style)) return 0;
    }}
    return 0;
  }}
  var NAMED = 'main, nav, aside, header, footer, section, h1, h2, h3, button, a, input, textarea, select, kbd, [role]';
  var all = document.querySelectorAll('*');
  function scrolling(value) {{ return value === 'auto' || value === 'scroll' ? 1 : 0; }}
  for (var i = 0; i < all.length; i++) {{
    var node = all[i];
    var style = getComputedStyle(node);
    var across = scrolling(style.overflowX);
    var down = scrolling(style.overflowY);
    if (!node.matches(NAMED) && across === 0 && down === 0) continue;
    if (node.checkVisibility && !node.checkVisibility()) continue;
    var rect = node.getBoundingClientRect();
    var depth = 0;
    for (var up = node.parentElement; up; up = up.parentElement) depth++;
    var parent = -1;
    for (var s = 0; s < seen.length; s++) {{ if (seen[s].contains(node)) parent = s; }}
    seen.push(node);
    out.push([
      node.tagName,
      node.getAttribute('role') || '-',
      encodeURIComponent(named(node).slice(0, 80)) || '-',
      Math.round(rect.left), Math.round(rect.top),
      Math.round(rect.width), Math.round(rect.height),
      depth, parent,
      across, down,
      firstMark(node), underlined(node)
    ].join(' '));
  }}
  document.getElementById('{SINK}').textContent = out.join(' ; ');
  document.getElementById('{CONDITIONS}').textContent =
    (matchMedia('(forced-colors: active)').matches ? '1' : '0') + ' ' +
    (getComputedStyle(document.documentElement).colorScheme || 'normal').replace(/\s+/g, '-');
}}, {SETTLE_MS});
</script>
"#
    )
}
