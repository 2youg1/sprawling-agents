// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a progress bar says before it is drawn. A total of zero means
// the end is not known yet: the bar then says it is busy instead of
// claiming a fraction it does not have. The three ARIA values follow
// that one question exactly - the lower bound is always there, the
// upper bound and the current value appear only when there is an end
// to be at, and the bar is busy in every other case
// (`client/spec/Views/Parts.lean`, the progress row of §7-1).

export interface ProgressProps {
  // The accessible name of the bar, already in the person's language.
  readonly label: string;
  readonly done: number;
  // Zero or less means the end is unknown.
  readonly total: number;
}

// Spread on the element drawn as the bar's track.
export interface BarWire {
  readonly role: "progressbar";
  readonly "aria-label": string;
  readonly "aria-valuemin": 0;
  readonly "aria-valuemax"?: number;
  readonly "aria-valuenow"?: number;
  readonly "aria-busy": "true" | "false";
}

// The two numbers the bar was drawn from, when there is an end.
export interface Reached {
  readonly done: number;
  readonly total: number;
  // How much of the track is filled, from 0 to 1.
  readonly share: number;
}

export interface ProgressLook {
  readonly bar: BarWire;
  // Undefined while the end is unknown: the look draws the busy track.
  readonly reached: Reached | undefined;
}

export function lookOf({ label, done, total }: ProgressProps): ProgressLook {
  if (total <= 0) {
    return {
      bar: { role: "progressbar", "aria-label": label, "aria-valuemin": 0, "aria-busy": "true" },
      reached: undefined,
    };
  }
  return {
    bar: {
      role: "progressbar",
      "aria-label": label,
      "aria-valuemin": 0,
      "aria-valuemax": total,
      "aria-valuenow": done,
      "aria-busy": "false",
    },
    reached: { done, total, share: Math.min(Math.max(done / total, 0), 1) },
  };
}
