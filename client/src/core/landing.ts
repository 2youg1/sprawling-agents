// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where the work a page dispatched actually started. A dispatch at a
// bare building opens a room the city names (`<building>/<room>`), so
// the page that sent it is not where the run is; this is the one place
// that recognises the run as the one sent from here (client-SPEC 12-3).

import type { Address, RunId } from "../wire";
import type { RunBelief } from "./belief";

// A dispatch this page sent, as it stood the moment it went out.
export interface Sent {
  readonly from: Address;
  readonly task: string;
  readonly known: ReadonlySet<RunId>;
}

export type Landing =
  | { readonly kind: "pending" }
  | { readonly kind: "here" }
  | { readonly kind: "elsewhere"; readonly run: RunId; readonly addr: Address };

export function sentFrom(from: Address, task: string, runs: Readonly<Record<RunId, RunBelief>>): Sent {
  return { from, task, known: new Set(Object.values(runs).map((run) => run.run)) };
}

export function landingOf(_sent: Sent, _runs: Readonly<Record<RunId, RunBelief>>): Landing {
  return { kind: "pending" };
}
