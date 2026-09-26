// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How many runs this city cancelled. The wire carries no count of what
// one halt froze, so this counts the runs whose own freeze says
// `cancelled`, which is what a halt writes; belief keeps the count as it
// folds, so the page's halt line does not walk the table on every
// record (client-SPEC 12-5).

import type { RunBelief } from "./shape";

function cancelled(run: RunBelief | undefined): boolean {
  return run?.doing.kind === "frozen" && run.doing.completion === "cancelled";
}

// The count over the whole table, for a write that replaces the table.
export function cancelledOf(runs: Readonly<Record<string, RunBelief>>): number {
  return Object.values(runs).filter((run) => cancelled(run)).length;
}
