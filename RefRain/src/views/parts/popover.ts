// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key } from "../../core/lang";

export interface PopoverRow {
  readonly id: string;
  readonly label: string;
  readonly secondary?: string | undefined;
  readonly chosen?: boolean | undefined;
}

export interface PopoverBinding {
  readonly keys: (event: KeyboardEvent) => boolean;
  readonly controls: readonly string[];
  readonly pointColumn: (columnId: string) => void;
}

export interface PopoverColumn {
  readonly id: string;
  readonly label: Key;
  readonly rows: readonly PopoverRow[];
}
