// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { AxCode, AxError } from "../wire";

export type Urgency = "needs_you" | "ordinary";

const URGENCY: Readonly<Record<AxCode, Urgency>> = {
  E_APPROVAL_PENDING: "needs_you",

  E_MODEL_UNCHOSEN: "needs_you",
  E_CREDENTIAL_MISSING: "needs_you",

  E_PROVIDER_ACCOUNTS_EXHAUSTED: "needs_you",
  E_ENDPOINT_DIALECT_UNSUPPORTED: "needs_you",

  E_CONFIG_INVALID: "needs_you",

  E_BUDGET_EXHAUSTED: "needs_you",

  E_PLAN_MISSING: "needs_you",

  E_WIRE_MISMATCH: "needs_you",

  E_CAS_CORRUPT: "needs_you",
  E_STORAGE_FATAL: "needs_you",
  E_LEDGER_HELD: "needs_you",
  E_LOG_VERSION_UNSUPPORTED: "needs_you",
  E_HISTORY_UNPROVEN: "needs_you",
  E_DIGEST_SUSPECT: "needs_you",

  E_AUTH_REQUIRED: "needs_you",
  E_PAIRING_REFUSED: "needs_you",

  E_PATH_NOT_FOUND: "ordinary",
  E_TOOL_UNKNOWN: "ordinary",
  E_TOOL_UNAVAILABLE: "ordinary",
  E_INVALID_ARGS: "ordinary",
  E_OUTSIDE_WRITE_DOMAIN: "ordinary",
  E_VERSION_CONFLICT: "ordinary",
  E_GATE_DENIED: "ordinary",
  E_TIMEOUT: "ordinary",
  E_PROVIDER: "ordinary",
  E_EVIDENCE_MISSING: "ordinary",
  E_LOOP_SUSPECTED: "ordinary",
  E_LOCATOR_INVALID: "ordinary",
  E_SANDBOX_DENIED: "ordinary",
  E_BUSY: "ordinary",
  E_DRAFT_STALE: "ordinary",
  E_GOAL_CONFLICT: "ordinary",
  E_TAINTED_ACTION: "ordinary",
  E_REPAIR_BUSY: "ordinary",
  E_DELEGATION_DEPTH: "ordinary",
  E_APPROVAL_DENIED: "ordinary",
  E_CROSS_BUILDING_DENIED: "ordinary",
  E_WORKTREE_BUSY: "ordinary",
  E_BROWSER_UNAVAILABLE: "ordinary",
  E_SECRET_EGRESS: "ordinary",
  E_DISCARD_IRREVERSIBLE: "ordinary",
  E_BACKPRESSURE_SHED: "ordinary",
  E_TOOL_OUTCOME_UNKNOWN: "ordinary",
};

export function urgencyOf(error: Pick<AxError, "code">): Urgency {
  return URGENCY[error.code];
}

export type Box =
  | { readonly kind: "absent" }
  | { readonly kind: "holding" }
  | { readonly kind: "emptied"; readonly at: number; readonly by: "send" | "hand" };

export interface Attention {
  readonly box: Box;

  readonly visible: boolean;

  readonly returnedAt: number | null;
}

export type Moment = "sent" | "idle" | "returned";

export const IDLE_MS = 5_000;

export const RETURNED_MS = 1_000;

export function momentOf(attention: Attention, now: number): Moment | null {
  if (!attention.visible) return null;
  if (attention.returnedAt !== null && now - attention.returnedAt <= RETURNED_MS) return "returned";
  const box = attention.box;
  switch (box.kind) {
    case "absent":
      return "idle";
    case "holding":
      return null;
    case "emptied":
      if (box.by === "send") return "sent";
      return now - box.at >= IDLE_MS ? "idle" : null;
  }
}

export function deliverable(urgency: Urgency, attention: Attention, now: number): boolean {
  switch (urgency) {
    case "needs_you":
      return true;
    case "ordinary":
      return momentOf(attention, now) !== null;
  }
}

export function wakeAt(attention: Attention, now: number): number | null {
  const box = attention.box;
  if (!attention.visible || box.kind !== "emptied" || box.by !== "hand") return null;
  const due = box.at + IDLE_MS;
  return due > now ? due : null;
}
