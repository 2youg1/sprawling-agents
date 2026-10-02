// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What became of a person's unsaved work on one document: the draft the
// browser keeps, the save the page sent, and the city's receipt for it
// (client/Spec.lean §4-46, `crates/wire/Spec.lean` §8-72).
//
// **Only the city says "saved".** A save is a `PutRange` on the version
// it was made on; the city answers a refusal or nothing, and records a
// landed save as a `document_written` line carrying the command's key.
// Until that line is read the page says "saving", and a link that drops
// in between leaves the result unknown ("pending") until the same key
// is sent again, which the city answers with its first result.

import { Option, Schema } from "effect";

import { B3Hash } from "../wire";
import type { Address, AxError, Command, EventRecord, Query } from "../wire";
import { putRange } from "./commands/document";
import type { EditorChange, Positions } from "./document_pos";
import { textEdits } from "./document_pos";

// ------------------------------------------------------------ the draft

// A person's unsaved changes to one document, against the version they
// were made on, in that version's editor coordinates.
export interface Draft {
  readonly version: B3Hash;
  readonly changes: readonly EditorChange[];
}

// The place the preference door keeps a document's draft under. A colon
// is not in the address grammar, so no room's draft shares the name.
export function draftPlace(doc: Address): string {
  return `document:${doc}`;
}

// The stored form of a draft; an empty one is no row at all.
export function writeDraft(draft: Draft): string {
  return draft.changes.length === 0 ? "" : JSON.stringify(draft);
}

const Stored = Schema.Struct({
  version: B3Hash,
  changes: Schema.Array(Schema.Struct({ from: Schema.Int, to: Schema.Int, insert: Schema.String })),
});

// A stored draft, if it is one whose changes fit a version `length`
// editor positions long: in document order, apart, and inside it. A
// value that is not is no draft, because applying it would put text
// where the person never typed it.
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

// ------------------------------------------------------------ the save

// One save as it was sent: the document, the version it was made on,
// and the command itself, which is sent again whole - its key included
// - when the link comes back before the receipt did.
export interface Sent {
  readonly doc: Address;
  readonly baseline: B3Hash;
  readonly command: Command;
}

export function saveOf(doc: Address, positions: Positions, changes: readonly EditorChange[]): Sent {
  return { doc, baseline: positions.version, command: putRange(doc, positions.version, textEdits(positions, changes)) };
}

// How many of the newest ledger lines the page reads for a receipt
// while a save is in flight. The receipt is written as the save lands,
// and `history` is asked again on every record (`staleness.ts`), so it
// is among the newest unless a burst of this many lines landed between
// two asks.
const RECEIPT_PAGE = 64;

export const RECEIPT_QUERY: Query = { history: { before: null, limit: RECEIPT_PAGE } };

// The version a save left, read from the one line that carries its key.
export function receiptIn(records: readonly EventRecord[], sent: Sent): B3Hash | null {
  const idem = "put_range" in sent.command ? sent.command.put_range.idem : null;
  const line = records.find((record) => record.kind === "document_written" && record.data.idem === idem);
  return line === undefined ? null : Option.getOrNull(Schema.decodeUnknownOption(B3Hash)(line.data.version));
}

// ------------------------------------------------------------ the receipt

// Where a document's unsaved work stands: nothing unsaved (`clean`, or
// `saved` right after a receipt), unsaved (`draft`), sent and not yet
// receipted (`saving`), sent before the link dropped (`pending`), made
// on a version the city no longer holds (`conflict`, with the version it
// holds when the page knows it), or refused for another reason. While a
// save is in flight `edited` says whether the text moved since it left.
export type Receipt =
  | { readonly kind: "clean" }
  | { readonly kind: "draft" }
  | { readonly kind: "saving"; readonly sent: Sent; readonly edited: boolean }
  | { readonly kind: "pending"; readonly sent: Sent; readonly edited: boolean }
  | { readonly kind: "saved"; readonly version: B3Hash }
  | { readonly kind: "conflict"; readonly current: B3Hash | null }
  | { readonly kind: "refused"; readonly error: AxError };

// What can happen to it. `edited` says whether the text now differs from
// what the city holds as far as the page knows (from what was sent,
// while a save is in flight); `based` that the draft now stands on the
// version the city holds, rebased or discarded; `moved` that the city
// answered a version other than the draft's.
export type Happened =
  | { readonly kind: "edited"; readonly empty: boolean }
  | { readonly kind: "based"; readonly empty: boolean }
  | { readonly kind: "sent"; readonly sent: Sent }
  | { readonly kind: "lost" }
  | { readonly kind: "relinked" }
  | { readonly kind: "landed"; readonly version: B3Hash }
  | { readonly kind: "refusal"; readonly error: AxError }
  | { readonly kind: "moved"; readonly version: B3Hash };

// The city's spelling of the action a refused save failed at
// (`crates/documents/src/edit.rs`, `accounting::worker::commanding::saving`).
// A refusal carries no key (§8-2), so this is how one is told to be a
// save's.
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

// Nothing in flight: an edit decides whether there is a draft, a send
// starts a save, and a version the city moved to is a conflict only
// when there is a draft for it to conflict with.
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

// A save in flight waits for its own receipt or its own refusal. The
// version the city answers meanwhile may be this save landing, so it
// decides nothing; the receipt does.
function inFlight(receipt: Receipt & { readonly kind: "saving" | "pending" }, happened: Happened): Receipt {
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
