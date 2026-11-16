// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { RunBelief } from "./shape";

function cancelled(run: RunBelief | undefined): boolean {
  return run?.doing.kind === "frozen" && run.doing.completion === "cancelled";
}

export function cancelledOf(runs: Readonly<Record<string, RunBelief>>): number {
  return Object.values(runs).filter((run) => cancelled(run)).length;
}

export function recounted(count: number, was: RunBelief | undefined, run: RunBelief): number {
  return count - Number(cancelled(was)) + Number(cancelled(run));
}
