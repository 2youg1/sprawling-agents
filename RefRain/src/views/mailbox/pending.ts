// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { urgencyOf } from "../../core/deferral";
import type { Notice } from "../../core/belief";
import type { Key } from "../../core/lang";

export interface Stopped {
  readonly asks: readonly Notice[];

  readonly failures: readonly Notice[];
}

export function stoppedOf(notices: readonly Notice[]): Stopped {
  const stopped = [...notices]
    .reverse()
    .filter((notice) => urgencyOf(notice.error) === "needs_you");
  return {
    asks: stopped.filter((notice) => notice.error.code === "E_APPROVAL_PENDING"),
    failures: stopped.filter((notice) => notice.error.code !== "E_APPROVAL_PENDING"),
  };
}

export type PendingKind = "ask" | "failure" | "question" | "proposal";

export const WHY: Readonly<Record<PendingKind, Key>> = {
  ask: "mailbox_why_ask",
  failure: "mailbox_why_failure",
  question: "mailbox_why_question",
  proposal: "mailbox_why_proposal",
};
