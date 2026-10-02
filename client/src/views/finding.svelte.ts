// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Whether the file finder stands open, and which building it looks in
// (client/Spec.lean §4-62). Module state, so the shell's Accel-P and the
// palette's entry of the same name open the one finder.

import { MAYOR, buildingOf } from "../core/route";
import type { View } from "../core/route";
import type { Address } from "../wire";

let shown = $state(false);

export function finderShown(): boolean {
  return shown;
}

export function openFinder(): void {
  shown = true;
}

export function closeFinder(): void {
  shown = false;
}

// The building in front of the person: the room's building on a
// conversation, the building on its own page, and the Mayor's building
// on a page that stands over the whole city.
export function underOf(view: View): Address {
  switch (view.kind) {
    case "talk":
      return buildingOf(view.address);
    case "building":
      return buildingOf(view.address);
    case "city":
    case "run":
    case "setup":
    case "mcp":
    case "record":
    case "cost":
    case "registry":
    case "welcome":
    case "monitor":
    case "gallery":
      return buildingOf(MAYOR);
  }
}
