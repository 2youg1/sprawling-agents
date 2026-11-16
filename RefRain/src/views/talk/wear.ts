// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Phase } from "../runs/lineage";
import { PHASES } from "../runs/phase";

export type Stretch = readonly [number, number];

export interface Mark {
  readonly top: number;
  readonly height: number;
  readonly phase: Phase;
}

export interface TrackWire {
  readonly onpointerdown: (
    event: PointerEvent & { currentTarget: EventTarget & HTMLElement },
  ) => void;
  readonly onpointermove: (
    event: PointerEvent & { currentTarget: EventTarget & HTMLElement },
  ) => void;
}

export interface WearLook {
  readonly hint: string;
  readonly track: TrackWire;
  readonly read: readonly Stretch[];
  readonly thumb: Stretch;
  readonly marks: readonly Mark[];
}

export function worn(seen: readonly Stretch[], from: number, to: number): readonly Stretch[] {
  const apart = seen.filter(([a, b]) => b < from || a > to);
  const joined = seen.filter(([a, b]) => !(b < from || a > to));
  const start = Math.min(from, ...joined.map(([a]) => a));
  const end = Math.max(to, ...joined.map(([, b]) => b));
  return [...apart, [start, end] as const].sort((x, y) => x[0] - y[0]);
}

export function phaseNamed(word: string | undefined): Phase | undefined {
  return PHASES.find((each) => each === word);
}
