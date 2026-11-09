// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which pane of the building catalog the composer doors open. The
// catalog seat reads this once on mount and clears it, so a later
// visit without a door press starts on tools again.

export type CatalogFocus = "tools" | "skills" | "mcp" | "worktree" | "rules" | "models";

let focus = $state<CatalogFocus | null>(null);

export function focusCatalog(next: CatalogFocus): void {
  focus = next;
}

export function takeCatalogFocus(): CatalogFocus | null {
  const held = focus;
  focus = null;
  return held;
}
