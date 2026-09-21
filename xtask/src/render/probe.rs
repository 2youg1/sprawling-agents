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

use super::pass::Pass;

mod read;

pub(super) use read::{declared, elements};

/// The element the probe writes its measurements into.
pub(super) const SINK: &str = "sprawling-render";

/// The element the probe writes the conditions it drew under into.
pub(super) const CONDITIONS: &str = "sprawling-render-conditions";

/// The element the probe writes the page's own vocabulary into.
pub(super) const DECLARED: &str = "sprawling-render-declared";

/// What separates two records in a sink.
pub(super) const BETWEEN: &str = " ; ";

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
/// **Colour is taken through a one-pixel canvas, not parsed.** A
/// computed `color` in this client serialises as `oklch(…)`, and every
/// hand-written parser for the colour grammar is a second
/// implementation of a specification that grows. Painting the value
/// into a canvas and reading the four bytes back gives exactly what a
/// screen would receive, for every syntax the engine accepts, in three
/// lines.
///
/// **A colour behind an alpha is composited, and a colour behind
/// something this cannot model is refused.** A fill is reported as the
/// stack of backgrounds under it resolves; a background image, a blend
/// mode, a filter or a backdrop filter anywhere in that stack makes the
/// answer unknowable from the document, and the record then carries no
/// colour rather than a wrong one.
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
<pre id="{DECLARED}"></pre>
<script>
setTimeout(function () {{
  document.documentElement.dataset.theme = '{theme}';
  var out = [];
  var seen = [];
  var slate = document.createElement('canvas');
  slate.width = 1;
  slate.height = 1;
  var ink = slate.getContext('2d', {{ willReadFrequently: true }});
  function bytes(value) {{
    ink.clearRect(0, 0, 1, 1);
    ink.fillStyle = 'rgba(0, 0, 0, 0)';
    ink.fillStyle = value;
    ink.fillRect(0, 0, 1, 1);
    var read = ink.getImageData(0, 0, 1, 1).data;
    return [read[0], read[1], read[2], read[3] / 255];
  }}
  function over(top, under) {{
    var alpha = top[3] + under[3] * (1 - top[3]);
    if (alpha <= 0) return [0, 0, 0, 0];
    var mix = function (i) {{
      return (top[i] * top[3] + under[i] * under[3] * (1 - top[3])) / alpha;
    }};
    return [mix(0), mix(1), mix(2), alpha];
  }}
  function hex(colour) {{
    if (!colour || colour[3] < 0.999) return '-';
    var pair = function (i) {{ return ('0' + Math.round(colour[i]).toString(16)).slice(-2); }};
    return pair(0) + pair(1) + pair(2);
  }}
  function opaque(style) {{
    return style.backgroundImage === 'none' && style.mixBlendMode === 'normal'
      && style.filter === 'none' && style.backdropFilter === 'none';
  }}
  function stack(node) {{
    var acc = [0, 0, 0, 0];
    for (var up = node; up; up = up.parentElement) {{
      var style = getComputedStyle(up);
      if (!opaque(style)) return null;
      acc = over(acc, bytes(style.backgroundColor));
      if (acc[3] >= 0.999) return acc;
    }}
    return null;
  }}
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
  function writes(node) {{
    for (var k = 0; k < node.childNodes.length; k++) {{
      var child = node.childNodes[k];
      if (child.nodeType === 3 && (child.textContent || '').trim()) return true;
    }}
    return false;
  }}
  function run(node, style) {{
    if (!writes(node)) return '-';
    var glyph = bytes(style.color);
    var behind = glyph[3] >= 0.999 ? null : stack(node);
    var painted = glyph[3] >= 0.999 ? glyph : (behind ? over(glyph, behind) : null);
    var mark = style.overflowX === 'visible' ? 'spills'
      : (style.textOverflow === 'ellipsis' ? 'ellipsis' : 'clip');
    return [
      hex(painted),
      Math.round(parseFloat(style.fontSize) * 100),
      node.clientWidth, node.scrollWidth, mark
    ].join(',');
  }}
  function fill(node, style) {{
    return bytes(style.backgroundColor)[3] <= 0 ? '-' : hex(stack(node));
  }}
  var NAMED = 'main, nav, aside, header, footer, section, h1, h2, h3, button, a, input, textarea, select, kbd, [role]';
  var all = document.querySelectorAll('*');
  function spilling(value) {{
    if (value === 'auto' || value === 'scroll') return 'scrolls';
    return value === 'visible' ? 'shows' : 'clips';
  }}
  for (var i = 0; i < all.length; i++) {{
    var node = all[i];
    var style = getComputedStyle(node);
    var across = spilling(style.overflowX);
    var down = spilling(style.overflowY);
    var frame = node.matches(NAMED) || across !== 'shows' || down !== 'shows';
    if (!frame && !writes(node)) continue;
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
      across, down, frame ? 'frame' : 'words',
      firstMark(node), underlined(node),
      fill(node, style),
      encodeURIComponent((node.getAttribute('class') || '').slice(0, 160)) || '-',
      run(node, style)
    ].join(' '));
  }}
  var ruler = document.createElement('div');
  ruler.style.position = 'absolute';
  ruler.style.visibility = 'hidden';
  document.body.appendChild(ruler);
  function length(value) {{
    ruler.style.width = '0px';
    ruler.style.width = value;
    return Math.round(parseFloat(getComputedStyle(ruler).width) * 100);
  }}
  var words = [];
  var root = getComputedStyle(document.documentElement);
  for (var w = 0; w < root.length; w++) {{
    var word = root[w];
    if (word.indexOf('--color-') === 0) {{
      words.push(word + ' ' + hex(bytes(root.getPropertyValue(word))));
    }} else if (word.indexOf('--text-') === 0 || word.indexOf('--spacing-') === 0) {{
      words.push(word + ' ' + length(root.getPropertyValue(word)));
    }}
  }}
  ruler.remove();
  document.getElementById('{SINK}').textContent = out.join('{BETWEEN}');
  document.getElementById('{DECLARED}').textContent = words.join('{BETWEEN}');
  document.getElementById('{CONDITIONS}').textContent =
    (matchMedia('(forced-colors: active)').matches ? '1' : '0') + ' ' +
    (getComputedStyle(document.documentElement).colorScheme || 'normal').replace(/\s+/g, '-');
}}, {SETTLE_MS});
</script>
"#
    )
}
