// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get, readonly, writable } from "../core/store";

export type Arrival = "keys" | "touch";

interface PaletteState {
  readonly shown: boolean;
  readonly arrived: Arrival;
}

const showing = writable<PaletteState>({ shown: false, arrived: "keys" });
export const paletteState = readonly(showing);

export function paletteShown(state = get(showing)): boolean {
  return state.shown;
}

export function paletteArrival(state = get(showing)): Arrival {
  return state.arrived;
}

export function openPalette(arrival: Arrival): void {
  showing.set({ arrived: arrival, shown: true });
}

export function closePalette(): void {
  showing.update((state) => ({ ...state, shown: false }));
}

export function togglePalette(): void {
  showing.update((state) => ({ arrived: "keys", shown: !state.shown }));
}
