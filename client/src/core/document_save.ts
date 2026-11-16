// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import { B3Hash } from "../wire";
import type { Address, AxError, Command, EventRecord, Query } from "../wire";
import { putRange } from "./commands/document";
import type { EditorChange, Positions } from "./document_pos";
import { textEdits } from "./document_pos";

export interface Draft {
  readonly version: B3Hash;
  readonly changes: readonly EditorChange[];
}

export function draftPlace(doc: Address): string {
  return `document:${doc}`;
}

export function writeDraft(draft: Draft): string {
  return draft.changes.length === 0 ? "" : JSON.stringify(draft);
}

const Stored = Schema.Struct({
  version: B3Hash,
  changes: Schema.Array(Schema.Struct({ from: Schema.Int, to: Schema.Int, insert: Schema.String })),
});

export function readDraft(stored: string, length: number): Draft | null {
  const parsed = Option.getOrNull(Schema.decodeOption(Schema.fromJsonString(Stored))(stored));
  if (parsed === null) return null;
  let reached = 0;
  for (const change of parsed.changes) {
    if (change.from < reached || change.to < change.from || change.to > length) return null;
    reached = change.to;
  }
  return parsed;
}

export interface Sent {
  readonly doc: Address;
  readonly baseline: B3Hash;
  readonly command: Command;
}

export function saveOf(doc: Address, positions: Positions, changes: readonly EditorChange[]): Sent {
  return {
    doc,
    baseline: positions.version,
    command: putRange(doc, positions.version, textEdits(positions, changes)),
  };
}

const RECEIPT_PAGE = 64;

export const RECEIPT_QUERY: Query = { history: { before: null, limit: RECEIPT_PAGE } };

export function receiptIn(records: readonly EventRecord[], sent: Sent): B3Hash | null {
  const idem = "put_range" in sent.command ? sent.command.put_range.idem : null;
  const line = records.find(
    (record) => record.kind === "document_written" && record.data.idem === idem,
  );
  return line === undefined
    ? null
    : Option.getOrNull(Schema.decodeUnknownOption(B3Hash)(line.data.version));
}

export type Receipt =
  | { readonly kind: "clean" }
  | { readonly kind: "draft" }
  | { readonly kind: "saving"; readonly sent: Sent; readonly edited: boolean }
  | { readonly kind: "pending"; readonly sent: Sent; readonly edited: boolean }
  | { readonly kind: "saved"; readonly version: B3Hash }
  | { readonly kind: "conflict"; readonly current: B3Hash | null }
  | { readonly kind: "refused"; readonly error: AxError };

export type Happened =
  | { readonly kind: "edited"; readonly empty: boolean }
  | { readonly kind: "based"; readonly empty: boolean }
  | { readonly kind: "sent"; readonly sent: Sent }
  | { readonly kind: "lost" }
  | { readonly kind: "relinked" }
  | { readonly kind: "landed"; readonly version: B3Hash }
  | { readonly kind: "refusal"; readonly error: AxError }
  | { readonly kind: "moved"; readonly version: B3Hash };

const SAVE_ACTION = "save a document";

export function advance(receipt: Receipt, happened: Happened): Receipt {
  switch (receipt.kind) {
    case "clean":
    case "saved":
    case "draft":
    case "refused":
      return atRest(receipt, happened);
    case "saving":
    case "pending":
      return inFlight(receipt, happened);
    case "conflict":
      return happened.kind === "based" ? unsaved(happened.empty) : receipt;
  }
}

function unsaved(empty: boolean): Receipt {
  return empty ? { kind: "clean" } : { kind: "draft" };
}

function atRest(receipt: Receipt, happened: Happened): Receipt {
  switch (happened.kind) {
    case "edited":
    case "based":
      return unsaved(happened.empty);
    case "sent":
      return { kind: "saving", sent: happened.sent, edited: false };
    case "moved":
      return receipt.kind === "draft" || receipt.kind === "refused"
        ? { kind: "conflict", current: happened.version }
        : receipt;
    case "lost":
    case "relinked":
    case "landed":
    case "refusal":
      return receipt;
  }
}

function inFlight(
  receipt: Receipt & { readonly kind: "saving" | "pending" },
  happened: Happened,
): Receipt {
  switch (happened.kind) {
    case "edited":
      return { ...receipt, edited: !happened.empty };
    case "landed":
      return receipt.edited ? { kind: "draft" } : { kind: "saved", version: happened.version };
    case "refusal":
      if (happened.error.action !== SAVE_ACTION) return receipt;
      return happened.error.code === "E_VERSION_CONFLICT"
        ? { kind: "conflict", current: null }
        : { kind: "refused", error: happened.error };
    case "lost":
      return { kind: "pending", sent: receipt.sent, edited: receipt.edited };
    case "relinked":
      return { kind: "saving", sent: receipt.sent, edited: receipt.edited };
    case "based":
    case "sent":
    case "moved":
      return receipt;
  }
}
