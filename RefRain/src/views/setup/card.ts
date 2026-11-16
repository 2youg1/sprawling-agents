// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";

export interface ReceiptWire {
  readonly role: "status";
}

export interface ReceiptLook {
  readonly wire: ReceiptWire;

  readonly said: string | undefined;
}

export interface CardLook {
  readonly title: string;
  readonly note?: string | undefined;

  readonly constraint?: string | undefined;

  readonly receipt?: ReceiptLook | undefined;
  readonly children: Snippet;
}

const RECEIPT: ReceiptWire = { role: "status" };

export function receiptOf(landed: boolean, saved: string): ReceiptLook {
  return { wire: RECEIPT, said: landed ? saved : undefined };
}
