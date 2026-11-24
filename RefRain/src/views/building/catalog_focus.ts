// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get, readonly, writable } from "../../core/store";

export type CatalogFocus = "tools" | "skills" | "mcp" | "worktree" | "rules" | "models";

const focus = writable<CatalogFocus | null>(null);
export const catalogFocus = readonly(focus);

export function focusCatalog(next: CatalogFocus): void {
  focus.set(next);
}

export function takeCatalogFocus(): CatalogFocus | null {
  const held = get(focus);
  focus.set(null);
  return held;
}
