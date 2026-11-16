// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";

import type { PopoverBinding } from "../parts/popover";
import type { PickerLook } from "./picker_look";
import type { PermissionsLook } from "./policy";

export type RowDraws = "everything" | "notice";

export type RowMenu = "policy" | "model" | null;

export type RowStarts = "open" | "closed" | "model" | "provider" | "level";

export function menuOf(starts: RowStarts): RowMenu {
  switch (starts) {
    case "open":
      return "policy";
    case "model":
    case "provider":
    case "level":
      return "model";
    case "closed":
      return null;
  }
}

export function sameBinding(held: PopoverBinding | null, next: PopoverBinding): boolean {
  return (
    held !== null &&
    held.keys === next.keys &&
    held.pointColumn === next.pointColumn &&
    held.controls.join(" ") === next.controls.join(" ")
  );
}

export interface SettingsRowLook {
  readonly catalog: Snippet | undefined;

  readonly facts: Snippet | undefined;

  readonly notLive: string | undefined;
  readonly model: PickerLook | undefined;
  readonly permissions: PermissionsLook | undefined;
}
