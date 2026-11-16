// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const MACHINE_ASKS_FOR_LESS = "(prefers-reduced-motion: reduce)";

export function motionOff(root: HTMLElement): boolean {
  const held = root.dataset.motion;
  if (held === "off") return true;
  if (held === "on") return false;
  return window.matchMedia(MACHINE_ASKS_FOR_LESS).matches;
}
