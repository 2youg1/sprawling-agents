// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The audit `keys.ts`' defaults are held to (client/Spec.lean D96): the
// chords the four browsers give a function of their own.

import { accel, accelShift, spell } from "./keys";
import type { Bound, Chord } from "./keys";

// The chords a browser gives a function of its own, from the official
// shortcut lists of Chrome (support.google.com/chrome/answer/157179),
// Edge (support.microsoft.com, "Keyboard shortcuts in Microsoft Edge"),
// Firefox (the `<key>` table of `browser/base/content/browser-sets.inc.xhtml`,
// which its support page is written from) and Safari
// (support.apple.com/guide/safari/cpsh003). No default may sit on one;
// a person may still bind one, because a person who never prints may
// want Accel-P for the finder.
export const BROWSER_KEEPS: readonly Chord[] = [
  // Select a tab: all four.
  ...["1", "2", "3", "4", "5", "6", "7", "8", "9"].map(accel),
  // Settings on macOS: Chrome, Firefox, Safari.
  accel(","),
  // Search from the address bar: Chrome, Edge, Firefox.
  accel("k"),
  // Downloads: Chrome, Edge, Firefox.
  accel("j"),
  // Bookmarks sidebar: Firefox.
  accel("b"),
  // Print: all four.
  accel("p"),
  // Stop loading on macOS: Firefox, Safari.
  accel("."),
  // Search open tabs: Firefox; add-ons: Firefox on Windows and Linux,
  // and on macOS under E; search in the sidebar: Edge; Collections:
  // Edge; switch text direction: Firefox.
  accelShift("a"),
  accelShift("f"),
  accelShift("e"),
  accelShift("y"),
  accelShift("x"),
];

// Whether a browser gives this chord a function of its own.
export function browserKeeps(held: Bound): boolean {
  return held !== null && BROWSER_KEEPS.some((kept) => spell(kept) === spell(held));
}
