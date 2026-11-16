// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


export type Arrival = "keys" | "touch";

let shown = $state(false);
let arrived = $state<Arrival>("keys");

export function paletteShown(): boolean {
  return shown;
}

export function paletteArrival(): Arrival {
  return arrived;
}

export function openPalette(arrival: Arrival): void {
  arrived = arrival;
  shown = true;
}

export function closePalette(): void {
  shown = false;
}


export function togglePalette(): void {
  arrived = "keys";
  shown = !shown;
}
