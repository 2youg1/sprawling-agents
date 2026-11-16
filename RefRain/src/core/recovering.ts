// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { Option, Schema } from "effect";

import type { Key } from "./lang";
import { fill } from "./lang";
import { MAYOR } from "./route";
import { readRunId } from "./run_id";
import { Address } from "../wire";
import type { AxCode, AxError } from "../wire";


export type FormRoom = "mayor" | "building";

export type Recovery =
  | { readonly kind: "command"; readonly spelled: string }
  | { readonly kind: "reconnect"; readonly verb: Key }
  | { readonly kind: "settings"; readonly verb: Key }
  
  
  | { readonly kind: "reload"; readonly verb: Key }
  | {
      readonly kind: "form";
      readonly label: Key;
      readonly words: Key;
      readonly room: FormRoom;
    };


export interface Form {
  readonly room: Address;
  readonly draft: string;
}

const NEW: Recovery = { kind: "command", spelled: "/new" };
const FORK: Recovery = { kind: "command", spelled: "/fork" };
const STOP: Recovery = { kind: "command", spelled: "/stop" };


const AGAIN: Recovery = { kind: "reconnect", verb: "link_retry" };


const SETTINGS: Recovery = { kind: "settings", verb: "act_go_settings" };
const RELOAD: Recovery = { kind: "reload", verb: "link_reload" };
const ASK_PLAN: Recovery = {
  kind: "form",
  label: "act_ask_plan",
  words: "form_ask_plan",
  room: "mayor",
};

const RECOVERIES: Readonly<Record<AxCode, readonly Recovery[]>> = {
  
  
  E_CONFIG_INVALID: [NEW, FORK],
  
  
  E_MODEL_UNCHOSEN: [SETTINGS],
  
  
  E_PROVIDER_ACCOUNTS_EXHAUSTED: [SETTINGS],
  
  
  E_BUSY: [STOP],
  E_WORKTREE_BUSY: [STOP],
  E_REPAIR_BUSY: [STOP],
  
  
  E_LOOP_SUSPECTED: [STOP],
  
  
  E_TIMEOUT: [AGAIN],
  E_BACKPRESSURE_SHED: [AGAIN],
  
  
  E_WIRE_MISMATCH: [RELOAD],
  
  
  E_PLAN_MISSING: [ASK_PLAN],
  
  
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
  E_AUTH_REQUIRED: [],
  E_PAIRING_REFUSED: [],
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
  E_HISTORY_UNPROVEN: [],
  E_SECRET_EGRESS: [],
  E_DISCARD_IRREVERSIBLE: [],
  E_TOOL_OUTCOME_UNKNOWN: [],
};


export function formOf(room: FormRoom, subject: string, words: string): Option.Option<Form> {
  
  const colon = subject.indexOf(":");
  const name = subject.slice(colon + 1).trim();
  if (colon < 0 || name === "") {
    return Option.none();
  }
  return Option.map(Schema.decodeOption(Address)(subject.slice(0, colon)), (building) => ({
    room: room === "mayor" ? MAYOR : building,
    draft: fill(words, { building, name }),
  }));
}


export function recoveryFor(error: Pick<AxError, "code" | "subject">): readonly Recovery[] {
  const offered = RECOVERIES[error.code];
  return Option.isSome(readRunId(error.subject))
    ? offered
    : offered.filter((each) => each !== STOP);
}


export function linkRecovery(code: AxCode): Recovery {
  return RECOVERIES[code].find((recovery) => recovery.kind === "reload") ?? AGAIN;
}
