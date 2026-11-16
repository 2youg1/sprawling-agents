// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import type { Key } from "../core/lang";
import { buildingDestination, buildingOf } from "../core/route";
import type { Template } from "../core/commands";
import { Address } from "../wire";
import type { BuildingProgress } from "../wire";

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
  const destination = buildingDestination(addr, buildings);
  switch (destination.kind) {
    case "building":
      return { kind: "refused", why: "raise_taken" };
    case "adopt":
    case "build":
      return { kind: destination.kind, addr };
  }
}

export const OFFERED: readonly Exclude<Template, "hall">[] = ["minimal", "confidential"];

export function waiting(buildings: readonly BuildingProgress[]): readonly Address[] {
  return buildings.filter((each) => each.adoptable).map((each) => each.addr);
}

export function sourceOf(written: string): string | null {
  const text = written.trim();
  return text === "" ? null : text;
}

export function sourceRefusal(written: string): Key | null {
  const text = written.trim();
  if (text === "") return null;
  const absolute = text.startsWith("/") || text.startsWith("\\\\") || /^[A-Za-z]:[\\/]/.test(text);
  return absolute ? null : "raise_source_relative";
}

export function fillable(
  buildings: readonly BuildingProgress[],
  typed: string,
): readonly Address[] {
  return waiting(buildings).filter((folder) => folder !== typed.trim());
}
