// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which refusals wait in the mailbox's deciding section, and why each
// thing there is there (client/Spec.lean §4-49). A refusal is pending
// only when `core/deferral.ts` calls it `needs_you`: a tool call the
// model was refused and went on from is `ordinary`, so it reads in the
// notices section and never as a card that waits on the User.
import { urgencyOf } from "../../core/deferral";
import type { Notice } from "../../core/belief";
import type { Key } from "../../core/lang";

export interface Stopped {
  // Doors waiting on the User's own hand.
  readonly asks: readonly Notice[];
  // Every other refusal that stops the work until the User acts.
  readonly failures: readonly Notice[];
}

// The pending refusals among `notices`, newest first.
export function stoppedOf(notices: readonly Notice[]): Stopped {
  const stopped = [...notices].reverse().filter((notice) => urgencyOf(notice.error) === "needs_you");
  return {
    asks: stopped.filter((notice) => notice.error.code === "E_APPROVAL_PENDING"),
    failures: stopped.filter((notice) => notice.error.code !== "E_APPROVAL_PENDING"),
  };
}

// The four kinds of thing deciding holds.
export type PendingKind = "ask" | "failure" | "question" | "proposal";

// The line each card carries under its head: which event or refusal
// put it here, and which rule keeps it here until the User acts.
export const WHY: Readonly<Record<PendingKind, Key>> = {
  ask: "mailbox_why_ask",
  failure: "mailbox_why_failure",
  question: "mailbox_why_question",
  proposal: "mailbox_why_proposal",
};
