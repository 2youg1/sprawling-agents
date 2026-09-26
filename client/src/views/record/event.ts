// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One ledger record as a line a person reads: what happened, in their
// language, and the few fields of its payload that say something to a
// person. The rest of the payload - the chain's hashes, the idempotency
// key, the position a record was written after - is what makes the
// ledger verifiable, and it stays one fold away as the record wrote it.

import type { Key } from "../../core/lang";
import type { EventKind, Payload } from "../../wire";

// What each kind of record says happened. Built from the kind rather
// than listed, and typed as a `Key`, so a kind the wire adds without a
// sentence in `lang.json` fails to compile instead of drawing a blank.
export function whatHappened(kind: EventKind): Key {
  return `event_${kind}`;
}

// A fact is one field a person reads by name.
export interface Fact {
  readonly name: string;
  readonly value: string;
}

// The fields that hold the ledger together rather than say what
// happened. Named, because the shape of their values is not enough: an
// `after` is a number like any count.
const MACHINERY: ReadonlySet<string> = new Set([
  "after", "before", "idem", "prev", "hash", "digest", "v", "seq", "oid", "tool_use_id",
]);

// A value no person reads: a hash, a run id, an object id.
const OPAQUE = /^(?:[0-9a-f]{16,}|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$/iu;

// How many facts a line carries; the rest of the record is behind the
// fold, whole.
const MOST = 4;

export function factsOf(data: Payload): readonly Fact[] {
  return Object.entries(data)
    .flatMap(([name, value]): Fact[] => {
      if (MACHINERY.has(name) || name.endsWith("_hash")) return [];
      if (typeof value === "number" || typeof value === "boolean") return [{ name, value: String(value) }];
      if (typeof value !== "string" || value.trim() === "" || OPAQUE.test(value)) return [];
      return [{ name, value }];
    })
    .slice(0, MOST);
}
