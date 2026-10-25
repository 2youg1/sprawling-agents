// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the raise page decides without the page (client/Spec.lean 4-50b):
// what a typed address would become, and which templates a person is
// offered. Whether the address names a folder already there is the
// city's answer (`BuildingProgress.adoptable`), read here and never
// worked out again from a listing of the disk.

import { Option, Schema } from "effect";

import type { Key } from "../core/lang";
import { buildingOf } from "../core/route";
import type { Template } from "../core/commands";
import { Address } from "../wire";
import type { BuildingProgress } from "../wire";

// What raising the typed address would do: nothing yet, nothing at all
// with the reason, or a building laid out empty or drawn over a folder.
export type Raising =
  | { readonly kind: "blank" }
  | { readonly kind: "refused"; readonly why: Key }
  | { readonly kind: "build"; readonly addr: Address }
  | { readonly kind: "adopt"; readonly addr: Address };

export function raisingOf(typed: string, buildings: readonly BuildingProgress[]): Raising {
  const text = typed.trim();
  if (text === "") return { kind: "blank" };
  const addr = Option.getOrNull(Schema.decodeOption(Address)(text));
  if (addr === null) return { kind: "refused", why: "raise_unreadable" };
  if (buildingOf(addr) !== addr) return { kind: "refused", why: "raise_room" };
  const held = buildings.find((each) => each.addr === addr);
  if (held === undefined) return { kind: "build", addr };
  return held.adoptable ? { kind: "adopt", addr } : { kind: "refused", why: "raise_taken" };
}

// The templates a person raises a building from. City Hall's is the
// city's own and is raised with the city, so it is not offered.
export const OFFERED: readonly Exclude<Template, "hall">[] = ["minimal", "confidential"];

// The folders waiting to be adopted.
export function waiting(buildings: readonly BuildingProgress[]): readonly Address[] {
  return buildings.filter((each) => each.adoptable).map((each) => each.addr);
}

// The waiting folders the page offers to fill in: the one already in the
// box is not offered again, since the box says it.
export function fillable(buildings: readonly BuildingProgress[], typed: string): readonly Address[] {
  return waiting(buildings).filter((folder) => folder !== typed.trim());
}
