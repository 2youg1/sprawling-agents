// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where a list that opens over a control stands: which side of its
// anchor it opens on, and how far the list scrolls to show the row the
// cursor is on. `popover.svelte` and `combobox.svelte` both read this
// file, so the two parts follow one rule for each. The rules and their
// proofs are `reveal` and `opensOn` in
// `client/spec/Views/Parts/Popover.lean`; `layer.test.ts` checks this
// implementation against the properties proved there.
//
// The two pure rules come first. The two readers below them take the
// numbers out of the page and are the only lines here that touch the
// DOM.

// Above or below the anchor. Each part has a side it prefers: the
// composer's lists open above the box at the bottom of the page, a
// form's combobox opens below its trigger.
export type Side = "above" | "below";

// Whether a layer keeps the side its part prefers, as the model says it.
export type Opening = "preferred" | "other";

// The scroll offset that shows the row from `top` to `bottom` (both in
// the list's content, in pixels) inside a viewport `height` tall that
// is scrolled to `scroll`: the nearest edge moves, so the other rows
// stay where they were, and a row taller than the viewport shows its
// first line.
export function reveal(top: number, bottom: number, scroll: number, height: number): number {
  if (top < scroll || height < bottom - top) return top;
  if (scroll + height < bottom) return bottom - height;
  return scroll;
}

// Which side a layer opens on. `seen` says the anchor stands inside the
// box that clips it, `here` and `there` are the room the preferred and
// the other side have in that box, `height` is how far the layer reaches
// past its anchor. The layer keeps its side whenever it fits there, and
// moves only to a side with more room.
export function opensOn(seen: boolean, here: number, there: number, height: number): Opening {
  return seen && here < height && here < there ? "other" : "preferred";
}

// The side `layer` opens on, measured where it is drawn now, which is
// on the side `preferred` names: its anchor is the positioned box it is
// placed against. Layout offsets rather than client boxes measure how
// far the layer reaches, so an entrance still translating the layer
// does not change the answer.
export function sideFor(layer: HTMLElement, preferred: Side): Side {
  const anchor = layer.offsetParent;
  if (!(anchor instanceof HTMLElement)) return preferred;
  const box = anchor.getBoundingClientRect();
  const clip = clipOf(anchor);
  const below = clip.bottom - box.bottom;
  const above = box.top - clip.top;
  const reach =
    preferred === "below"
      ? layer.offsetTop + layer.offsetHeight - anchor.offsetHeight
      : -layer.offsetTop;
  // Edges included: an anchor with no height of its own, the first
  // thing in a box that clips, stands on that box's top edge.
  const seen = box.bottom >= clip.top && box.top <= clip.bottom;
  const opening =
    preferred === "below" ? opensOn(seen, below, above, reach) : opensOn(seen, above, below, reach);
  if (opening === "preferred") return preferred;
  return preferred === "below" ? "above" : "below";
}

// Scrolls `list` so `row` is in view, by `reveal`. Neither the focus
// nor any other scroller moves.
export function revealIn(list: HTMLElement, row: HTMLElement): void {
  const viewport = list.getBoundingClientRect();
  const bounds = row.getBoundingClientRect();
  const top = bounds.top - viewport.top + list.scrollTop;
  list.scrollTop = reveal(top, top + bounds.height, list.scrollTop, viewport.height);
}

interface Span {
  readonly top: number;
  readonly bottom: number;
}

// The box whose edges cut what `node` holds: the nearest ancestor that
// does not let its contents show past it, or the window. Measured
// against that box rather than the window, because a settings page's
// combobox stands in a pane that scrolls, and the edge that cuts its
// list is the pane's.
function clipOf(node: HTMLElement): Span {
  for (let up = node.parentElement; up !== null && up !== document.body; up = up.parentElement) {
    if (getComputedStyle(up).overflowY !== "visible") {
      const box = up.getBoundingClientRect();
      return { top: box.top, bottom: box.bottom };
    }
  }
  return { top: 0, bottom: window.innerHeight };
}
