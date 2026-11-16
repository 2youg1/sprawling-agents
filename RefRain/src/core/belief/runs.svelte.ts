// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The run table as `$state`: every run a reactive object keyed by its
// `RunId`, so a token written into one run wakes the readers of that
// run's field and nobody else, and a view iterating the table re-runs
// only when a run arrives or leaves. A record keeps the table indexable
// the way every view already reads it; a `SvelteMap` would give the same
// per-key grain and cost every reader a migration to `get`.

import type { RunBelief } from "./shape";

export function runTable(held: Record<string, RunBelief>): Record<string, RunBelief> {
  const table = $state(held);
  return table;
}
