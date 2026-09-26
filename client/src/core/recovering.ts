// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a person can do about a refusal, as one table read by every
// notice surface (client-SPEC 4-35). A drawer and a toast that each
// mapped codes to actions of their own would give one refusal two
// answers; this file is the one answer, and it draws nothing.
//
// **A command is spelled as itself** (design 4-10): `spelled` is both
// the deed and the label a control for it carries, in the two languages
// alike. `reconnect` has no command spelling - it acts on the link the
// page already has - so its label is a word, and the word's key travels
// with the action to keep the choice of that word in one home.
//
// An empty row is a decision: most refusals name their own recovery in
// the sentence the city wrote (`AxError.recovery`) and offer nothing a
// person can run from the notice. Every code has a row, so a new code
// is a decision here rather than a silence.

import type { Key } from "./lang";
import type { AxCode } from "../wire";

export type Recovery =
  | { readonly kind: "command"; readonly spelled: string }
  | { readonly kind: "reconnect"; readonly verb: Key }
  | { readonly kind: "settings"; readonly verb: Key };

const NEW: Recovery = { kind: "command", spelled: "/new" };
const FORK: Recovery = { kind: "command", spelled: "/fork" };
const STOP: Recovery = { kind: "command", spelled: "/stop" };
// The word for asking the link to try again lives here rather than at
// two notice surfaces (`link_retry` in `lang.json`).
const AGAIN: Recovery = { kind: "reconnect", verb: "link_retry" };
// Choosing a model happens on the settings page and nowhere else, so
// the one thing a notice can do about an unchosen model is take the
// person there.
const SETTINGS: Recovery = { kind: "settings", verb: "act_go_settings" };

const RECOVERIES: Readonly<Record<AxCode, readonly Recovery[]>> = {
  // The room's model is frozen against this session: leave the room, or
  // branch from the run that already got somewhere (the assignment
  // behind this table).
  E_CONFIG_INVALID: [NEW, FORK],
  // Nothing is chosen for this kind of model yet: a new room or a
  // branch would meet the same refusal, and the settings page is where
  // a provider is attached and a model picked.
  E_MODEL_UNCHOSEN: [SETTINGS],
  // Something is occupying the place this refusal is about, and the
  // only way past it is to stop what is running there.
  E_BUSY: [STOP],
  E_WORKTREE_BUSY: [STOP],
  E_REPAIR_BUSY: [STOP],
  // The watchdog suspects a loop. Stopping the run is the intervention
  // a person has; the sentence says what was suspected.
  E_LOOP_SUSPECTED: [STOP],
  // Silence, a stopped link, and a shed load share one lever: asking
  // the link to try. This is the same door the refusal corner opens.
  E_TIMEOUT: [AGAIN],
  E_WIRE_MISMATCH: [AGAIN],
  E_BACKPRESSURE_SHED: [AGAIN],
  // Everything else: the refusal's own sentence is the guidance, and
  // no command helps from the notice.
  E_PATH_NOT_FOUND: [],
  E_TOOL_UNKNOWN: [],
  E_TOOL_UNAVAILABLE: [],
  E_INVALID_ARGS: [],
  E_OUTSIDE_WRITE_DOMAIN: [],
  E_VERSION_CONFLICT: [],
  E_GATE_DENIED: [],
  E_BUDGET_EXHAUSTED: [],
  E_PROVIDER: [],
  E_EVIDENCE_MISSING: [],
  E_LOCATOR_INVALID: [],
  E_SANDBOX_DENIED: [],
  E_DRAFT_STALE: [],
  E_GOAL_CONFLICT: [],
  E_TAINTED_ACTION: [],
  E_DELEGATION_DEPTH: [],
  E_APPROVAL_PENDING: [],
  E_APPROVAL_DENIED: [],
  E_CROSS_BUILDING_DENIED: [],
  E_DIGEST_SUSPECT: [],
  E_CREDENTIAL_MISSING: [],
  E_CAS_CORRUPT: [],
  E_STORAGE_FATAL: [],
  E_BROWSER_UNAVAILABLE: [],
  E_ENDPOINT_DIALECT_UNSUPPORTED: [],
  E_LOG_VERSION_UNSUPPORTED: [],
  E_LEDGER_HELD: [],
  E_SECRET_EGRESS: [],
  E_DISCARD_IRREVERSIBLE: [],
  E_TOOL_OUTCOME_UNKNOWN: [],
};

// The actions a person can take about one refusal, in the order a
// notice offers them.
export function recoveryFor(code: AxCode): readonly Recovery[] {
  return RECOVERIES[code];
}
