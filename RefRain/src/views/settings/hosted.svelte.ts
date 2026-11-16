// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { DEFAULT_VIEW, toFragment } from "../../core/route";
import { markEndAtFrame, markStart } from "../../core/timing";
import type { SetupGroup, View } from "../../core/route";
import { FIRST_GROUP } from "./tree";

let beneath = $state.raw<View>(DEFAULT_VIEW);

let last = $state.raw<SetupGroup>(FIRST_GROUP);


let pushed = false;

let opener: HTMLElement | null = null;
let closing = false;

export function panelBeneath(): View {
  return beneath;
}

export function panelGroup(view: { readonly group?: SetupGroup }): SetupGroup {
  return view.group ?? last;
}


export function hostSettled(was: View, now: View, arrived: boolean): HTMLElement | null {
  if (now.kind === "setup") {
    if (now.group !== undefined) last = now.group;
    if (was.kind === "setup" && was.group !== now.group) markEndAtFrame("settings_switch");
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


export function pickGroup(group: SetupGroup): void {
  markStart("settings_switch");
  location.replace(toFragment({ kind: "setup", group }));
}


export function closePanel(): void {
  closing = true;
  if (pushed) history.back();
  else location.replace(toFragment(beneath));
}
