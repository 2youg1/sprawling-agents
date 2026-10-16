// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// This module keeps the surface `models.tsx` had - the table of probed
// rows and the choice of which model fills each role reach a caller
// through one specifier - so the components are re-exported here beside
// the row the table hands back, the shape `parts/table.ts` established.
// The row and the props are written here rather than in the component
// because the lint lane resolves no named export of a `.svelte` module
// for a `.svelte` reader, and the shapes must not be written down
// twice.

import type { Stated } from "../../core/commands";
import type { ModelFact } from "../../core/probed";
import type { ModelTag } from "../../wire";
import type { TableAbout } from "./model_rows";

export { default as ModelChoice } from "./models.svelte";
export { default as ModelTable } from "./model_table.svelte";

// One ticked row: the model, the window and the ceiling a person typed
// and the input kinds they stated, and the role it is to fill if any.
export interface ModelRow {
  readonly id: string;
  readonly stated: Stated;
  readonly tag: ModelTag | null;
}

export interface ModelTableProps {
  readonly served: readonly ModelFact[];
  // What the table's rows are asked about, or `null` while the form
  // names no endpoint the city could be asked about.
  readonly about: TableAbout | null;
  readonly onChosen: (rows: readonly ModelRow[]) => void;
}
