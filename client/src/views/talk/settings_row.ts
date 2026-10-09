// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the settings row under the box decides before anything is drawn
// (docs/frontend-method.md §7I, client/Spec.lean §4-60): which menu a
// fixture draws open, and the whole value `settings_row.look.svelte`
// draws. The row is a seat and a look (client D95): `settings_row.svelte`
// holds which menu is open and the drawn elements; the model picker's
// half is `picker_look.ts` and the permissions entry's half `policy.ts`.

import type { Snippet } from "svelte";

import type { PopoverBinding } from "../parts/popover";
import type { PickerLook } from "./picker_look";
import type { PermissionsLook } from "./policy";

// What the row draws: everything before a session starts, and only the
// notice that a sentence the link did not take was kept in the box once
// one has (`talk_not_live`).
export type RowDraws = "everything" | "notice";

// The menu open on the row. The workspace chip's menu is the pill's own.
export type RowMenu = "policy" | "model" | null;

// The menu a fixture draws open; every screen starts closed.
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

// Whether a binding the popover hands over again is the one already
// held. The popover hands its binding over each time its props change,
// as a new object; holding each new object would change the row's look,
// which changes the popover's props again, without end.
export function sameBinding(held: PopoverBinding | null, next: PopoverBinding): boolean {
  return (
    held !== null &&
    held.keys === next.keys &&
    held.pointColumn === next.pointColumn &&
    held.controls.join(" ") === next.controls.join(" ")
  );
}

// Everything the look is given.
export interface SettingsRowLook {
  // The workspace chip and the sandbox, drawn by their own seats.
  readonly facts: Snippet | undefined;
  // Said when a sentence the link did not take is kept in the box.
  readonly notLive: string | undefined;
  readonly model: PickerLook | undefined;
  readonly permissions: PermissionsLook | undefined;
}
