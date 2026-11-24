// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get, readonly, writable } from "../../core/store";

import { DEFAULT_VIEW, toFragment } from "../../core/route";
import { markEndAtFrame, markStart } from "../../core/timing";
import type { SetupGroup, View } from "../../core/route";
import { FIRST_GROUP } from "./tree";

const panes = writable<{ beneath: View; last: SetupGroup }>({
  beneath: DEFAULT_VIEW,
  last: FIRST_GROUP,
});
export const setupPlacement = readonly(panes);

let pushed = false;

const SETTINGS_KEY = "workspace-settings-key";
const ACTIONS_KEY = "workspace-actions-key";
let requestedOpener: HTMLElement | null = null;

export function rememberSettingsOpener(control: HTMLElement): void {
  requestedOpener = control;
}

function restoreFocus(from: HTMLElement | null): boolean {
  if (takeFocus(from)) return true;
  if (from?.id !== SETTINGS_KEY) return false;
  return (
    takeFocus(document.getElementById(SETTINGS_KEY)) ||
    takeFocus(document.getElementById(ACTIONS_KEY))
  );
}

function takeFocus(element: HTMLElement | null): boolean {
  if (
    element === null ||
    !element.isConnected ||
    element.getClientRects().length === 0 ||
    element.closest("[hidden], [inert]") !== null
  )
    return false;
  element.focus({ preventScroll: true });
  return element.ownerDocument.activeElement === element;
}

let opener: HTMLElement | null = null;
let closing = false;

export function panelBeneath(held = get(panes)): View {
  return held.beneath;
}

export function panelGroup(view: { readonly group?: SetupGroup }, held = get(panes)): SetupGroup {
  return view.group ?? held.last;
}

export function hostSettled(was: View, now: View, arrived: boolean): (() => boolean) | null {
  if (now.kind === "setup") {
    const group = now.group;
    if (group !== undefined) panes.update((held) => ({ ...held, last: group }));
    if (was.kind === "setup" && was.group !== now.group) markEndAtFrame("settings_switch");
    if (was.kind !== "setup") {
      pushed = arrived;
      opener =
        requestedOpener ??
        (document.activeElement instanceof HTMLElement ? document.activeElement : null);
      requestedOpener = null;
    }
    return null;
  }
  panes.update((held) => ({ ...held, beneath: now }));
  const from = opener;
  const back = closing ? () => restoreFocus(from) : null;
  opener = null;
  requestedOpener = null;
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
  else location.replace(toFragment(panelBeneath()));
}
