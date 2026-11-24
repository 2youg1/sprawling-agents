// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { writable, type Writable } from "../../core/store";
import type { Ui } from "../../ui";
import type { ApprovalItem, AxError, Command } from "../../wire";

type Answer = Extract<Command, { readonly approve: unknown }>;

export type Attempt =
  | {
      readonly kind: "pending" | "unknown";
      readonly commands: readonly Answer[];
      readonly at: number;
    }
  | { readonly kind: "refused"; readonly error: AxError; readonly commands: readonly Answer[] };

const bySurface = new WeakMap<Ui, Writable<ReadonlyMap<string, Attempt>>>();

export function attemptsFor(surface: Ui): Writable<ReadonlyMap<string, Attempt>> {
  const found = bySurface.get(surface);
  if (found !== undefined) return found;
  const kept = writable<ReadonlyMap<string, Attempt>>(new Map());
  bySurface.set(surface, kept);
  return kept;
}

export function keepAttempt(
  surface: Writable<ReadonlyMap<string, Attempt>>,
  group: string,
  attempt: Attempt,
): void {
  surface.update((current) => new Map(current).set(group, attempt));
}

export function stillOpen(attempt: Attempt, items: readonly ApprovalItem[]): readonly Answer[] {
  const watching = new Set(items.map((item) => item.id));
  return attempt.commands.filter((command) => watching.has(command.approve.item));
}

export function seenApprovals(
  surface: Writable<ReadonlyMap<string, Attempt>>,
  items: readonly ApprovalItem[],
): void {
  surface.update((current) => {
    const kept = new Map(current);
    for (const [key, attempt] of current)
      if (stillOpen(attempt, items).length === 0) kept.delete(key);
    return kept.size === current.size ? current : kept;
  });
}
