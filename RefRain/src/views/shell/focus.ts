// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export function focusComposer(): boolean {
  const editor = document.querySelector<HTMLElement>(
    "#workspace-conversation textarea, main textarea",
  );
  if (editor?.closest("[inert]") !== null) return false;
  editor.focus();
  return document.activeElement === editor;
}

export function focusTitle(): void {
  const main = document.getElementById("main");
  const target = main?.querySelector<HTMLElement>("h1") ?? main;
  if (target != null) {
    target.tabIndex = -1;
    target.focus();
  }
}

export function focusRoute(): void {
  const target = document.querySelector<HTMLElement>("#main [data-route-focus]");
  if (target === null) focusTitle();
  else target.focus();
}

export function focusNavigation(): void {
  document.querySelector<HTMLElement>("#workspace-navigation [data-navigation-heading]")?.focus();
}
