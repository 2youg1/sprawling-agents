// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import { Address } from "../wire";
import type { HaltScope } from "../wire";

export const CITY = "city";
const BUILDING = "building";
const WORKSHOP = "workshop";
const AT = ":";

const readAddress = Schema.decodeOption(Address);

export function scopeOf(spelled: string | null): HaltScope | null {
  if (spelled === null) return null;
  if (spelled === CITY) return CITY;
  const cut = spelled.indexOf(AT);
  if (cut < 0) return null;
  const kind = spelled.slice(0, cut);
  const addr = Option.getOrNull(readAddress(spelled.slice(cut + 1)));
  if (addr === null) return null;
  if (kind === BUILDING) return { building: addr };
  if (kind === WORKSHOP) return { workshop: addr };
  return null;
}

export function sameScope(left: HaltScope, right: HaltScope): boolean {
  if (left === CITY || right === CITY) return left === right;
  if ("building" in left && "building" in right) return left.building === right.building;
  if ("workshop" in left && "workshop" in right) return left.workshop === right.workshop;
  return false;
}

export function buildingIsShut(halted: readonly HaltScope[], addr: Address): boolean {
  return halted.some((held) => held !== CITY && "building" in held && held.building === addr);
}

export function cityIsShut(halted: readonly HaltScope[]): boolean {
  return halted.includes(CITY);
}
