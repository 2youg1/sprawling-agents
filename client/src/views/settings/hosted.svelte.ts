// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where the settings panel stands in the shell (client-SPEC 7L). The
// panel opens over a page rather than in place of one: the page the
// person was on, or the conversation with the Mayor when the panel's own
// address was the first thing opened. Open and closed are the address
// bar's to say, so every move here is a move of the address bar, and the
// shell reads the result on `hashchange` like any other page.

import { DEFAULT_VIEW, toFragment } from "../../core/route";
import type { SetupGroup, View } from "../../core/route";
import { FIRST_GROUP } from "./tree";

let beneath = $state.raw<View>(DEFAULT_VIEW);
// The group drawn last, where a bare `#/setup` reopens the panel.
let last = $state.raw<SetupGroup>(FIRST_GROUP);
// Whether this page pushed the panel's address itself, so closing can
// step back off it rather than push a second copy of the page beneath.
let pushed = false;
// The control that opened the panel, which takes the focus back.
let opener: HTMLElement | null = null;
let closing = false;

export function panelBeneath(): View {
  return beneath;
}

export function panelGroup(view: { readonly group?: SetupGroup }): SetupGroup {
  return view.group ?? last;
}

// Reads one view the shell settled on, after `was`. Answers the control
// the focus goes back to when this view closed the panel, else `null`.
export function hostSettled(was: View, now: View, arrived: boolean): HTMLElement | null {
  if (now.kind === "setup") {
    if (now.group !== undefined) last = now.group;
    if (was.kind !== "setup") {
      pushed = arrived;
      opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    }
    return null;
  }
  beneath = now;
  const back = closing && opener?.isConnected === true ? opener : null;
  closing = false;
  return back;
}

// A group picked in the panel replaces the address, so the back button
// leaves the panel rather than walking back through every group.
export function pickGroup(group: SetupGroup): void {
  location.replace(toFragment({ kind: "setup", group }));
}

// Closing steps back off the panel's address when this page pushed it,
// and otherwise puts the page beneath in its place (client-SPEC 7-7).
export function closePanel(): void {
  closing = true;
  if (pushed) history.back();
  else location.replace(toFragment(beneath));
}
