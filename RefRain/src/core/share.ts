// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const WHOLE_PPB = 1_000_000_000;

export function fraction(ppb: number): number {
  return ppb / WHOLE_PPB;
}

export function percent(ppb: number): number {
  return Math.round(ppb / (WHOLE_PPB / 100));
}
