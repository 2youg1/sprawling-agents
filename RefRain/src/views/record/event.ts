// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key } from "../../core/lang";
import type { EventKind, Payload } from "../../wire";

export function whatHappened(kind: EventKind): Key {
  return `event_${kind}`;
}

export interface Fact {
  readonly name: string;
  readonly value: string;
}

const MACHINERY: ReadonlySet<string> = new Set([
  "after",
  "before",
  "idem",
  "prev",
  "hash",
  "digest",
  "v",
  "seq",
  "oid",
  "tool_use_id",
]);

const OPAQUE = /^(?:[0-9a-f]{16,}|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$/iu;

const MOST = 4;

export function factsOf(data: Payload): readonly Fact[] {
  return Object.entries(data)
    .flatMap(([name, value]): Fact[] => {
      if (MACHINERY.has(name) || name.endsWith("_hash")) return [];
      if (typeof value === "number" || typeof value === "boolean")
        return [{ name, value: String(value) }];
      if (typeof value !== "string" || value.trim() === "" || OPAQUE.test(value)) return [];
      return [{ name, value }];
    })
    .slice(0, MOST);
}
