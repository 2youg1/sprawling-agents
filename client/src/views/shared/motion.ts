// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Whether this person asked the page to stop moving.
//
// `setup/appearance.tsx` writes the answer onto the root element as
// `data-motion`, and `theme.css` reads that same attribute to shorten
// every transition to one frame. The two animations a script starts by
// itself - the composer falling to the foot of the page, and the view
// transition between two routes - cannot be reached by that rule, so
// they ask here instead. Before this, they read the machine's
// preference directly and therefore overruled a person who had asked
// for movement on a machine that asks for less.
//
// `system` is the absence of an opinion, which is the one case where
// the machine answers. That is the reading `theme.css` gives the same
// three values.

// The only query a browser offers for this, spelled once.
const MACHINE_ASKS_FOR_LESS = "(prefers-reduced-motion: reduce)";

export function motionOff(root: HTMLElement): boolean {
  const held = root.dataset.motion;
  if (held === "off") return true;
  if (held === "on") return false;
  return window.matchMedia(MACHINE_ASKS_FOR_LESS).matches;
}
