// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key } from "./lang";

export interface Sample {
  readonly core_cpu_permille: number;
  readonly core_private_bytes: number;
  readonly core_working_set_bytes: number;
  readonly core_read_bytes: number;
  readonly core_written_bytes: number;
  readonly machine_cpu_permille: number;
  readonly machine_available_bytes: number;
  readonly volume_free_bytes: number;
  readonly ledger_queue_depth: number;
  readonly durable_lag: number;
  readonly relay_p50_nanos: number;
  readonly event_to_screen_p50_nanos: number;
  readonly queued_runs: number;
}

export interface Row {
  readonly label: Key;
  readonly reading: string;
  readonly curve: string;
}

export function sparkline(_values: readonly number[], _width: number): string {
  return "";
}

export function rows(_samples: readonly Sample[], _width: number): readonly Row[] {
  return [];
}
