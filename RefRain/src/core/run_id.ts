// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";
import type { Option } from "effect";

import { RunId as written } from "../wire";
import type { RunId } from "../wire";

const read = Schema.decodeOption(written);

export function readRunId(raw: string): Option.Option<RunId> {
  return read(raw);
}
