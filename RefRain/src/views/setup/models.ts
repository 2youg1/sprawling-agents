// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Stated } from "../../core/commands";
import type { ModelFact } from "../../core/probed";
import type { ModelTag } from "../../wire";
import type { TableAbout, TableState } from "./model_rows";

export { default as ModelChoice } from "./models.svelte";
export { default as ModelTable } from "./model_table.svelte";

export interface ModelRow {
  readonly id: string;
  readonly stated: Stated;
  readonly tag: ModelTag | null;
}

export interface ModelTableProps {
  readonly served: readonly ModelFact[];
  readonly about: TableAbout | null;
  readonly state?: TableState;
  readonly onChosen: (rows: readonly ModelRow[]) => void;
}
