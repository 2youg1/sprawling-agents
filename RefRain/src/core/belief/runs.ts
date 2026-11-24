// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { writable, type Readable, type Writable } from "../store";
import type { RunBelief } from "./shape";

type RunField = "saying" | "thinking";

const observers = new WeakMap<RunBelief, Partial<Record<RunField, Writable<string>>>>();

export function runText(run: RunBelief, field: RunField): Readable<string> {
  const fields = observers.get(run) ?? {};
  const stream = fields[field];
  if (stream !== undefined) return stream;
  const next = writable(run[field]);
  fields[field] = next;
  observers.set(run, fields);
  return next;
}

export function appendRunText(run: RunBelief, field: RunField, addition: string): void {
  run[field] += addition;
  observers.get(run)?.[field]?.set(run[field]);
}
