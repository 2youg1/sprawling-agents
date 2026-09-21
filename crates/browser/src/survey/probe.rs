// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a page says about itself, in one reading.
//!
//! **One measurement, two ways of carrying it back.** The render gate
//! dumps a page's DOM and reads three `<pre>` sinks out of it; a
//! resident surveying a page it opened cannot append anything to that
//! page and takes the same three strings back as the value of one
//! evaluation. The body below is the measurement, and it is written
//! once: two spellings of it would drift, and the gate and the tool
//! would then disagree about a page while both reported honestly.
//!
//! The body is plain ES5 on purpose. It is injected into whatever the
//! person had open, which is not always this client.

mod read;

pub use read::{declared, elements};

use super::{Page, PaintSource};

/// What one evaluation of [`evaluated`] came back with.
///
/// Three strings, because three is what the measurement produces and a
/// reader that took them apart differently on each side of the seam
/// would judge two different pages.
#[derive(serde::Deserialize)]
pub struct Read {
    /// One record per element the walk reached.
    pub sink: String,
    /// One record per word the root element declares.
    pub declared: String,
    /// Whether forced colours were active, and the colour scheme the
    /// root element computed to.
    pub conditions: String,
}

impl Read {
    /// The page these readings describe.
    ///
    /// `painted_by` is the caller's, because the two callers know it
    /// differently: a gate demanded a pass and asserts the page obeyed,
    /// while a resident has only what the page reports about itself.
    #[must_use]
    pub fn page(&self, painted_by: PaintSource) -> Page {
        Page {
            drawn: elements(&self.sink),
            declared: declared(&self.declared),
            painted_by,
        }
    }

    /// Whether the page reported forced colours, and the colour scheme
    /// it computed to.
    #[must_use]
    pub fn conditions(&self) -> Option<(&str, &str)> {
        let mut said = self.conditions.split_whitespace();
        match (said.next(), said.next()) {
            (Some(forced), Some(scheme)) => Some((forced, scheme)),
            _ => None,
        }
    }
}

/// What separates one record from the next in all three strings.
///
/// Semicolons with spaces around them, because a class attribute
/// contains neither and a colour never does.
pub const BETWEEN: &str = " ; ";

/// How long the page is given to settle before it is measured.
///
/// Fonts load, a transition finishes, and a layout that is measured
/// before either has settled reports a page nobody ever saw.
pub const SETTLE_MS: u32 = 1200;

/// The measurement, as a JavaScript function body that returns the
/// three strings a [`Page`](super::Page) is read from.
///
/// A theme is written onto the root element before anything is read, so
/// one page can be measured in each theme it offers. `None` measures
/// the page as it was found, which is the only honest reading of a page
/// somebody else opened.
#[must_use]
pub fn body(theme: Option<&str>) -> String {
    let forced = match theme {
        Some(named) => format!(
            "  document.documentElement.dataset.theme = '{named}';
"
        ),
        None => String::new(),
    };
    format!(
        r#"
{forced}
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
  return {{
    sink: out.join('{BETWEEN}'),
    declared: words.join('{BETWEEN}'),
    conditions: (matchMedia('(forced-colors: active)').matches ? '1' : '0') + ' ' +
      (getComputedStyle(document.documentElement).colorScheme || 'normal').replace(/\s+/g, '-')
  }};
"#
    )
}

/// The measurement as one expression a driver can evaluate, resolving
/// to the JSON of the three strings.
///
/// A promise rather than a value because the page is given time to
/// settle first; a driver that does not await it reads the promise
/// instead of the page.
#[must_use]
pub fn evaluated(theme: Option<&str>) -> String {
    let body = body(theme);
    format!(
        "new Promise(function (done) {{ setTimeout(function () {{ done(JSON.stringify((function          () {{{body}}})())); }}, {SETTLE_MS}); }})"
    )
}
