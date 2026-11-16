// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";

import type { Key } from "../../core/lang";
import type { Receipt } from "../../core/document_save";

export interface Place {
  readonly folder: string;
  readonly name: string;
}

export interface ReceiptWord {
  readonly key: Key;
  readonly alert: boolean;
}

export interface ReceiptWire {
  readonly role: "status";
}

export interface HeadLook {
  readonly place: Place;
  readonly version: string | null;
  readonly receipt: { readonly text: string; readonly alert: boolean } | null;
  readonly wire: ReceiptWire;
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
