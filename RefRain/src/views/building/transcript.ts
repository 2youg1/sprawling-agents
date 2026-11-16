// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { Option } from "effect";

import { readRunId } from "../../core/run_id";
import type { RunId } from "../../wire";


export function transcriptOf(name: string): RunId | null {
  const stem = name.replace(/\.jsonl$/, "");
  if (stem === name) return null;
  return Option.getOrNull(readRunId(stem));
}
