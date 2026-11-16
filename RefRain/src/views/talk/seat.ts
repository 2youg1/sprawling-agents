// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { RunId } from "../../wire";

export type Seat =
  | { readonly kind: "page" }
  | { readonly kind: "workbench"; readonly gauged: RunId | null }
  | { readonly kind: "specimen" };

export const PAGE: Seat = { kind: "page" };
export const SPECIMEN: Seat = { kind: "specimen" };

export function drawsAddress(seat: Seat): boolean {
  switch (seat.kind) {
    case "page":
    case "workbench":
      return true;
    case "specimen":
      return false;
  }
}

export type FrozenFacts = "head" | "elsewhere";

export function factsAt(seat: Seat, at: number, run: RunId): FrozenFacts {
  if (at !== 0) return "elsewhere";
  switch (seat.kind) {
    case "page":
    case "specimen":
      return "head";
    case "workbench":
      return seat.gauged === run ? "elsewhere" : "head";
  }
}
