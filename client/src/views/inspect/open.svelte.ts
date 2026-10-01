// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the right side shows (client-SPEC 7F): the one state every opener
// writes and the workspace reads, so a tool line, a commit row and a file
// in the world layer open the same side through the same door.

import type { Address, B3Hash, RunId, Seq } from "../../wire";

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

export type RightItem = ({ readonly kind: "call" } & CallItem) | ({ readonly kind: "document" } & DocumentItem);

let shown = $state<RightItem | null>(null);

export function rightItem(): RightItem | null {
  return shown;
}

export function openCall(call: CallItem): void {
  shown = { kind: "call", run: call.run, at: call.at };
}

export function openDocument(document: DocumentItem): void {
  shown = { kind: "document", building: document.building, path: document.path, version: document.version };
}

export function closeRight(): void {
  shown = null;
}
