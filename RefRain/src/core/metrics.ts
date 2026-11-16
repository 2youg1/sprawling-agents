// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { LINE_HEIGHT, TRACKING } from "../wire";

export type Measured =
  | { readonly kind: "cleared" }
  | { readonly kind: "set"; readonly perMille: number }
  | { readonly kind: "refused" };

const WIDEST = 2_147_483_647;

function perMilleOf(text: string): number | null {
  const trimmed = text.trim();
  if (!/^-?[0-9]+(\.[0-9]{1,3})?$/.test(trimmed)) return null;
  const perMille = Math.round(Number(trimmed) * 1000);
  return Number.isSafeInteger(perMille) && Math.abs(perMille) <= WIDEST ? perMille : null;
}

export function ratioOf(text: string, floor: number): Measured {
  if (text.trim() === "") return { kind: "cleared" };
  const perMille = perMilleOf(text);
  if (perMille === null || perMille < floor) return { kind: "refused" };
  return { kind: "set", perMille };
}

export function spelledRatio(perMille: number): string {
  return String(perMille / 1000);
}

export const LINE_HEIGHT_FLOOR = LINE_HEIGHT.min;
export const TRACKING_FLOOR = TRACKING.min;
