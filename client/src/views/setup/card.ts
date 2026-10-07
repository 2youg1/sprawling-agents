// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One card of a settings group that writes as the person moves its
// control (client/Spec.lean §4-36): a title, one line saying what it
// governs, the control, and a foot with the card's line of constraint
// and its save receipt. Every such card in `views/setup/` is drawn by
// `card.look.svelte`, so the cards of one group line up with each other
// and a replacement look changes all of them at once; the cards saved
// by a press are `settings/card.svelte`.

import type { Snippet } from "svelte";

// The live region the receipt is said in. It stands while no receipt
// shows, because a screen reader announces only a change inside a
// region that was already on the page.
export interface ReceiptWire {
  readonly role: "status";
}

export interface ReceiptLook {
  readonly wire: ReceiptWire;
  // The word said while the card's last change has just landed, already
  // in the person's language; `undefined` the rest of the time.
  readonly said: string | undefined;
}

// Everything the look is given.
export interface CardLook {
  // Already in the person's language, as every string below is.
  readonly title: string;
  readonly note?: string | undefined;
  // The rule the control holds to, such as a range, stated under it.
  readonly constraint?: string | undefined;
  // Absent on a card whose control writes somewhere with no receipt.
  readonly receipt?: ReceiptLook | undefined;
  readonly children: Snippet;
}

const RECEIPT: ReceiptWire = { role: "status" };

// The receipt of a card: `landed` is whether the last change that
// landed is this card's.
export function receiptOf(landed: boolean, saved: string): ReceiptLook {
  return { wire: RECEIPT, said: landed ? saved : undefined };
}
