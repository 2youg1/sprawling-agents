// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Address, B3Hash, GitOid, RunId, Seq } from "../../wire";


export interface CallItem {
  readonly run: RunId;
  readonly at: Seq;
}


export interface DocumentItem {
  readonly building: Address;
  readonly path: string;
  readonly version: B3Hash | null;
}


export interface ChangesItem {
  readonly base: GitOid;
  readonly head: GitOid;
}


export interface LetterItem {
  readonly doc: Address;
  readonly card: B3Hash;
}


export interface ContextItem {
  readonly run: RunId;
  readonly room: Address | null;
}

export type RightItem =
  | ({ readonly kind: "call" } & CallItem)
  | ({ readonly kind: "document" } & DocumentItem)
  | ({ readonly kind: "changes" } & ChangesItem)
  | ({ readonly kind: "letter" } & LetterItem)
  | ({ readonly kind: "context" } & ContextItem);


const KEPT = 8;


export const INSPECTOR = "data-inspector";

interface Held {
  readonly item: RightItem;
  readonly touched: number;
}

let held = $state<readonly Held[]>([]);
let clock = 0;
let opener: HTMLElement | null = null;
let back: (() => void) | null = null;

export function sameItem(a: RightItem | null, b: RightItem): boolean {
  if (a === null) return false;
  switch (a.kind) {
    case "call":
      return b.kind === "call" && a.run === b.run && a.at === b.at;
    case "document":
      return (
        b.kind === "document" &&
        a.building === b.building &&
        a.path === b.path &&
        a.version === b.version
      );
    case "changes":
      return b.kind === "changes" && a.base === b.base && a.head === b.head;
    case "letter":
      return b.kind === "letter" && a.doc === b.doc && a.card === b.card;
    case "context":
      return b.kind === "context" && a.run === b.run;
  }
}


export function itemKey(item: RightItem): string {
  switch (item.kind) {
    case "call":
      return `call ${item.run} ${String(item.at)}`;
    case "document":
      return `document ${item.building} ${item.path} ${item.version ?? ""}`;
    case "changes":
      return `changes ${item.base} ${item.head}`;
    case "letter":
      return `letter ${item.doc} ${item.card}`;
    case "context":
      return `context ${item.run}`;
  }
}

export function rightItem(): RightItem | null {
  return frontWhere(() => true);
}


export function frontWhere(pick: (item: RightItem) => boolean): RightItem | null {
  return (
    held.reduce<Held | null>(
      (front, each) =>
        pick(each.item) && (front === null || each.touched > front.touched) ? each : front,
      null,
    )?.item ?? null
  );
}


export function openItems(): readonly RightItem[] {
  return held.map((each) => each.item);
}

export function openCall(call: CallItem): void {
  rememberOpener();
  showItem({ kind: "call", run: call.run, at: call.at });
}

export function openDocument(document: DocumentItem): void {
  rememberOpener();
  showItem({
    kind: "document",
    building: document.building,
    path: document.path,
    version: document.version,
  });
}

export function openChanges(changes: ChangesItem): void {
  rememberOpener();
  showItem({ kind: "changes", base: changes.base, head: changes.head });
}


export function openLetter(letter: LetterItem, comeBack: () => void): void {
  opener = null;
  back = comeBack;
  showItem({ kind: "letter", doc: letter.doc, card: letter.card });
}

export function openContext(context: ContextItem): void {
  rememberOpener();
  showItem({ kind: "context", run: context.run, room: context.room });
}


export function showItem(item: RightItem): void {
  clock += 1;
  const touched = clock;
  
  
  const kept = held.some((each) => sameItem(each.item, item))
    ? held.map((each) => (sameItem(each.item, item) ? { item, touched } : each))
    : [...held, { item, touched }];
  const oldest = kept.length > KEPT ? kept.reduce((a, b) => (b.touched < a.touched ? b : a)) : null;
  held = oldest === null ? kept : kept.filter((each) => each !== oldest);
}

export function closeItem(item: RightItem): void {
  held = held.filter((each) => !sameItem(each.item, item));
  if (held.length === 0 || (item.kind === "letter" && back !== null)) giveFocusBack();
}

export function closeRight(): void {
  held = [];
  giveFocusBack();
}

function rememberOpener(): void {
  back = null;
  if (typeof document === "undefined") return;
  const active = document.activeElement;
  if (active instanceof HTMLElement && active.closest(`[${INSPECTOR}]`) === null) opener = active;
}

function giveFocusBack(): void {
  if (typeof document === "undefined") return;
  const active = document.activeElement;
  const ours =
    active === null || active === document.body || active.closest(`[${INSPECTOR}]`) !== null;
  const returning = back;
  if (ours && returning !== null) returning();
  else if (ours && opener?.isConnected === true) opener.focus();
  opener = null;
  back = null;
}
