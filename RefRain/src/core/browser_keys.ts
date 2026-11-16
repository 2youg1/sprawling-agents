// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { accel, accelShift, spell } from "./keys";
import type { Bound, Chord } from "./keys";


export const BROWSER_KEEPS: readonly Chord[] = [
  
  ...["1", "2", "3", "4", "5", "6", "7", "8", "9"].map(accel),
  
  accel(","),
  
  accel("k"),
  
  accel("j"),
  
  accel("b"),
  
  accel("p"),
  
  accel("."),
  
  
  accelShift("a"),
  accelShift("f"),
  accelShift("e"),
  accelShift("y"),
  accelShift("x"),
];


export function browserKeeps(held: Bound): boolean {
  return held !== null && BROWSER_KEEPS.some((kept) => spell(kept) === spell(held));
}
