// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What RefRain's head line is given, and the word its seat reads off the
// last save (client D95 shape: a seat, a look, and this file).

import type { Snippet } from "svelte";

import type { Key } from "../../core/lang";
import type { Receipt } from "../../core/document_save";

// Where a document stands: its folder, which starts with its building's
// name, and the file's own name. The look draws the folder faint and the name
// solid, so the name is what the eye finds first.
export interface Place {
  readonly folder: string;
  readonly name: string;
}

// The receipt's word and whether it asks for the person, by state; a
// document nobody touched, or one saved and not touched since a clean
// read, has no word.
export interface ReceiptWord {
  readonly key: Key;
  readonly alert: boolean;
}

// The bag spread on the receipt. The receipt is one live region that is
// always drawn, empty while there is nothing to say, so the first word a
// save leaves is announced: a region that appears together with its words
// is not heard.
export interface ReceiptWire {
  readonly role: "status";
}

export interface HeadLook {
  readonly place: Place;
  // The first seven characters of the version the editor stands on, or
  // null before the city has answered.
  readonly version: string | null;
  // The receipt's word, already in the person's language, or null.
  readonly receipt: { readonly text: string; readonly alert: boolean } | null;
  readonly wire: ReceiptWire;
  // The readings and the save, drawn by their own parts at the line's end.
  readonly actions: Snippet;
}

export function receiptWord(kind: Receipt["kind"]): ReceiptWord | null {
  switch (kind) {
    case "clean":
      return null;
    case "draft":
      return { key: "refrain_receipt_draft", alert: true };
    case "saving":
      return { key: "refrain_receipt_saving", alert: false };
    case "pending":
      return { key: "refrain_receipt_pending", alert: true };
    case "saved":
      return { key: "refrain_receipt_saved", alert: false };
    case "conflict":
      return { key: "refrain_receipt_conflict", alert: true };
    case "refused":
      return { key: "refrain_receipt_refused", alert: true };
  }
}
