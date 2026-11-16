// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { ApprovalId, ApprovalItem } from "../wire";

export const WARMUP_MS = 5_000;

export type Notifying = "off" | "on";

export type Focus = "focused" | "blurred";

export type Heard =
  | { readonly snapshot: "pending" }
  | { readonly snapshot: "taken"; readonly ids: ReadonlySet<ApprovalId> };

export const UNHEARD: Heard = { snapshot: "pending" };

export interface Scene {
  readonly notifying: Notifying;
  readonly focus: Focus;
  readonly elapsed: number;
  readonly watching: string | null;
}

export function notices(
  heard: Heard,
  items: readonly ApprovalItem[] | undefined,
  scene: Scene,
): readonly [Heard, readonly ApprovalItem[]] {
  if (items === undefined) return [heard, []];
  const ids = new Set(items.map((item) => item.id));
  const next: Heard = { snapshot: "taken", ids };
  switch (heard.snapshot) {
    case "pending":
      return [next, []];
    case "taken":
      return [next, open(scene) ? items.filter((item) => raises(heard.ids, item, scene)) : []];
  }
}

function open(scene: Scene): boolean {
  return scene.notifying === "on" && scene.focus === "blurred" && scene.elapsed >= WARMUP_MS;
}

function raises(known: ReadonlySet<ApprovalId>, item: ApprovalItem, scene: Scene): boolean {
  return !known.has(item.id) && item.actor !== scene.watching;
}
