// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The shell's frame as the browser lays it out (client/Spec.lean §4-52, client D30):
// how many columns its grid has, and how much of the page the person can
// see while a soft keyboard is open.

// One column or the twelve of the grid. `theme.css` decides which, where
// the grid is decided, and writes it as `--shell-columns`; a script that
// has to know asks the element, so the width the grid folds at has one
// spelling and it is a style.
export type Columns = "one" | "twelve";

// Calls `onColumns` with the columns `element` stands in now, and again
// whenever its size changes, until the returned function is called. An
// element outside any frame stands in twelve.
export function watchColumns(element: Element, onColumns: (columns: Columns) => void): () => void {
  const observer = new ResizeObserver(() => {
    onColumns(getComputedStyle(element).getPropertyValue("--shell-columns").trim() === "1" ? "one" : "twelve");
  });
  observer.observe(element);
  return () => {
    observer.disconnect();
  };
}

// Keeps `--viewport-block` and `--viewport-top` on the shell's `frame`
// equal to the visual viewport's height and its offset into the layout
// viewport, which `theme.css`'s `visual-viewport` stands the frame on,
// until the returned function is called. A phone's soft keyboard shrinks the visual viewport
// and leaves the layout viewport where it was, and a browser may pan the
// one inside the other to show the field being typed in; the shell
// follows both, so the composer, its send key and the edge keys stay
// above the keyboard. A pinch zoom shrinks the visual viewport too, and
// that one the shell does not follow: the height is counted at the
// page's own scale and the pan is left to the person, because a page
// that refitted itself to every pinch could never be looked at closer.
export function followViewport(frame: HTMLElement, viewport: VisualViewport): () => void {
  const follow = (): void => {
    frame.style.setProperty("--viewport-block", `${String(viewport.height * viewport.scale)}px`);
    frame.style.setProperty("--viewport-top", `${String(viewport.scale > 1 ? 0 : viewport.offsetTop)}px`);
  };
  follow();
  viewport.addEventListener("resize", follow);
  viewport.addEventListener("scroll", follow);
  return () => {
    viewport.removeEventListener("resize", follow);
    viewport.removeEventListener("scroll", follow);
    frame.style.removeProperty("--viewport-block");
    frame.style.removeProperty("--viewport-top");
  };
}
