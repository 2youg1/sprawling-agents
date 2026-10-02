// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The properties this module must hold are proved in `client/spec/Views/Inspect/Open.lean`.
//
// What the right side shows (docs/frontend-method.md §7F, client/Spec.lean §4-45): the one state every
// opener writes and the inspector reads, so a tool line, a commit row and
// a file in the world layer open the same side through the same door.
//
// **Several items are open at once, one tab each, and one of them is in
// front.** The front item is the one touched last; the inspector shows,
// in each of its two regions, the item of that region touched last, so
// opening a command does not hide the file above it.
//
// **Every open item stays mounted until it is closed**, which is what
// keeps its scroll position, its selection and an unsaved RefRain draft
// when another tab comes forward (roadmap §3-14). That costs one view per
// tab, so at most `KEPT` are open, and opening one more closes the one
// touched longest ago.
//
// **Focus goes back where it came from.** The element that had the focus
// when an item was opened from outside the inspector is remembered, and
// closing the inspector while the focus is inside it hands the focus back
// to that element if it is still on the page (7-7).

import type { Address, B3Hash, GitOid, RunId, Seq } from "../../wire";

// A call, named the way `RoundsAnswer` names one: the run it belongs to
// and the ledger sequence of the call itself (`Call.at`).
export interface CallItem {
  readonly run: RunId;
  readonly at: Seq;
}

// A document of a building, at a version the ledger recorded, or at the
// worktree's current text when `version` is `null`.
export interface DocumentItem {
  readonly building: Address;
  readonly path: string;
  readonly version: B3Hash | null;
}

// What moved between two commits: a commit picked in the workbench, read
// against its first parent (refrain §3-9).
export interface ChangesItem {
  readonly base: GitOid;
  readonly head: GitOid;
}

export type RightItem =
  | ({ readonly kind: "call" } & CallItem)
  | ({ readonly kind: "document" } & DocumentItem)
  | ({ readonly kind: "changes" } & ChangesItem);

// Tabs a person keeps open before the oldest is closed for them: enough
// for a file, its diff and two commands on each side of a comparison.
const KEPT = 8;

// The attribute the inspector's root carries, by which a closing hand
// knows the focus is its own to give back.
export const INSPECTOR = "data-inspector";

interface Held {
  readonly item: RightItem;
  readonly touched: number;
}

let held = $state<readonly Held[]>([]);
let clock = 0;
let opener: HTMLElement | null = null;

export function sameItem(a: RightItem | null, b: RightItem): boolean {
  if (a === null) return false;
  switch (a.kind) {
    case "call":
      return b.kind === "call" && a.run === b.run && a.at === b.at;
    case "document":
      return b.kind === "document" && a.building === b.building && a.path === b.path && a.version === b.version;
    case "changes":
      return b.kind === "changes" && a.base === b.base && a.head === b.head;
  }
}

// One spelling per item, for keying a tab or a region's view: two
// readings of the same item are one key.
export function itemKey(item: RightItem): string {
  switch (item.kind) {
    case "call":
      return `call ${item.run} ${String(item.at)}`;
    case "document":
      return `document ${item.building} ${item.path} ${item.version ?? ""}`;
    case "changes":
      return `changes ${item.base} ${item.head}`;
  }
}

export function rightItem(): RightItem | null {
  return frontWhere(() => true);
}

// The item touched last among those `pick` accepts: what one region of
// the inspector shows.
export function frontWhere(pick: (item: RightItem) => boolean): RightItem | null {
  return held.reduce<Held | null>((front, each) => (pick(each.item) && (front === null || each.touched > front.touched) ? each : front), null)?.item ?? null;
}

// Every open item, in the order it was opened: the order of the tabs.
export function openItems(): readonly RightItem[] {
  return held.map((each) => each.item);
}

export function openCall(call: CallItem): void {
  rememberOpener();
  showItem({ kind: "call", run: call.run, at: call.at });
}

export function openDocument(document: DocumentItem): void {
  rememberOpener();
  showItem({ kind: "document", building: document.building, path: document.path, version: document.version });
}

export function openChanges(changes: ChangesItem): void {
  rememberOpener();
  showItem({ kind: "changes", base: changes.base, head: changes.head });
}

// Bring an item forward, opening it when it is not open yet.
export function showItem(item: RightItem): void {
  clock += 1;
  const touched = clock;
  const kept = held.some((each) => sameItem(each.item, item))
    ? held.map((each) => (sameItem(each.item, item) ? { item: each.item, touched } : each))
    : [...held, { item, touched }];
  const oldest = kept.length > KEPT ? kept.reduce((a, b) => (b.touched < a.touched ? b : a)) : null;
  held = oldest === null ? kept : kept.filter((each) => each !== oldest);
}

export function closeItem(item: RightItem): void {
  held = held.filter((each) => !sameItem(each.item, item));
  if (held.length === 0) giveFocusBack();
}

export function closeRight(): void {
  held = [];
  giveFocusBack();
}

function rememberOpener(): void {
  if (typeof document === "undefined") return;
  const active = document.activeElement;
  if (active instanceof HTMLElement && active.closest(`[${INSPECTOR}]`) === null) opener = active;
}

function giveFocusBack(): void {
  if (typeof document === "undefined") return;
  const active = document.activeElement;
  const ours = active === null || active === document.body || active.closest(`[${INSPECTOR}]`) !== null;
  if (ours && opener?.isConnected === true) opener.focus();
  opener = null;
}
