// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export type Side = "above" | "below";

export type Opening = "preferred" | "other";

export function reveal(top: number, bottom: number, scroll: number, height: number): number {
  if (top < scroll || height < bottom - top) return top;
  if (scroll + height < bottom) return bottom - height;
  return scroll;
}

export function opensOn(seen: boolean, here: number, there: number, height: number): Opening {
  return seen && here < height && here < there ? "other" : "preferred";
}

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

  const seen = box.bottom >= clip.top && box.top <= clip.bottom;
  const opening =
    preferred === "below" ? opensOn(seen, below, above, reach) : opensOn(seen, above, below, reach);
  if (opening === "preferred") return preferred;
  return preferred === "below" ? "above" : "below";
}

export function roomOn(layer: HTMLElement, side: Side): number | null {
  const anchor = layer.offsetParent;
  if (!(anchor instanceof HTMLElement)) return null;
  const box = anchor.getBoundingClientRect();
  const clip = clipOf(anchor);
  return Math.max(0, side === "above" ? box.top - clip.top : clip.bottom - box.bottom);
}

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

function clipOf(node: HTMLElement): Span {
  for (let up = node.parentElement; up !== null && up !== document.body; up = up.parentElement) {
    if (getComputedStyle(up).overflowY !== "visible") {
      const box = up.getBoundingClientRect();
      return { top: box.top, bottom: box.bottom };
    }
  }
  return { top: 0, bottom: window.innerHeight };
}
