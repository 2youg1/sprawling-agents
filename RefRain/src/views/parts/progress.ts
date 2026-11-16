// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


export interface ProgressProps {
  
  readonly label: string;
  readonly done: number;
  
  readonly total: number;
}


export interface BarWire {
  readonly role: "progressbar";
  readonly "aria-label": string;
  readonly "aria-valuemin": 0;
  readonly "aria-valuemax"?: number;
  readonly "aria-valuenow"?: number;
  readonly "aria-busy": "true" | "false";
}


export interface Reached {
  readonly done: number;
  readonly total: number;
  
  readonly share: number;
}

export interface ProgressLook {
  readonly bar: BarWire;
  
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
