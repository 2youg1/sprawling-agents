// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get, readonly, writable } from "../core/store";

import { MAYOR, buildingOf } from "../core/route";
import type { View } from "../core/route";
import type { Address } from "../wire";

const shown = writable(false);
export const finderState = readonly(shown);

export function finderShown(): boolean {
  return get(shown);
}

export function openFinder(): void {
  shown.set(true);
}

export function closeFinder(): void {
  shown.set(false);
}

export function underOf(view: View): Address {
  switch (view.kind) {
    case "talk":
      return buildingOf(view.address);
    case "building":
      return buildingOf(view.address);
    case "raise":
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
