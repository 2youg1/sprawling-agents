// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export const REACHABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled]):not([type=hidden])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "summary",
  "[contenteditable]:not([contenteditable=false])",
  "[tabindex]:not([tabindex='-1'])",
].join(", ");

export interface Lens {
  readonly id: string;
  readonly label: string;
}

export function tabId(group: string, lens: string): string {
  return `${group}-tab-${lens}`;
}

export function panelId(group: string, lens: string): string {
  return `${group}-panel-${lens}`;
}

export function tabStop(index: number, selected: number): 0 | -1 {
  return index === Math.max(0, selected) ? 0 : -1;
}

export function panelStop(active: boolean, reachable: boolean): 0 | undefined {
  return active && !reachable ? 0 : undefined;
}

export function landing(total: number, at: number, key: string): number | undefined {
  const last = total - 1;
  if (total === 0) return undefined;
  switch (key) {
    case "ArrowRight":
      return at === last ? 0 : at + 1;
    case "ArrowLeft":
      return at <= 0 ? last : at - 1;
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return undefined;
  }
}
