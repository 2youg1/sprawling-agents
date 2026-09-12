// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this page believes about the city, folded forward from what the
// server pushes. It holds the little that has to move at the speed of
// the stream - which runs exist, what each is doing right now, the text
// a model is still saying - and nothing that a query answers better.
// Reading one payload is `channels::reading`'s rule; the two readings
// used here (task, tool name and subject) copy that rule's field names
// and nothing else.

import { createStore, produce } from "solid-js/store";

import { readProbed } from "./probed";
import type { Probed } from "./probed";

import type {
  Address,
  AxError,
  CityAnswer,
  Delta,
  EventKind,
  EventRecord,
  LogLine,
  RunId,
  Seq,
  TimeMs,
} from "../wire";

// What one run is doing, in the words the city page and the room page
// both read.
export type Doing =
  | { readonly kind: "thinking" }
  | { readonly kind: "calling"; readonly tool: string; readonly subject: string | null }
  | { readonly kind: "waiting" }
  | { readonly kind: "frozen"; readonly completion: string | null };

// Where a message typed right now will land.
//
// A steer is consumed at a phase boundary, so "it was sent" and "it was
// heard" are not one moment: while a tool call is out, the run is inside
// a system call and the words wait for it to come back. Spelling all
// three the same way tells a person they are in a conversation when they
// are in a queue, which is the one thing a streaming page must not say.
export type Sending = "dispatch" | "steer" | "queued";

// No run, or a frozen one, means the next message opens work rather than
// interrupting it.
export function sendingInto(doing: Doing | undefined): Sending {
  if (doing === undefined) return "dispatch";
  switch (doing.kind) {
    case "frozen":
      return "dispatch";
    case "thinking":
      return "steer";
    // Blocked: inside a tool call, or stopped at an approval nobody has
    // answered. Neither reaches a safe point until it is over.
    case "calling":
    case "waiting":
      return "queued";
  }
}

export interface RunBelief {
  readonly run: RunId;
  readonly addr: Address | null;
  readonly started: TimeMs | null;
  readonly task: string | null;
  readonly lastSeq: Seq;
  readonly lastKind: EventKind;
  readonly doing: Doing;
  // What the model has said in the call that is still going. Cleared
  // when the call returns, because the record then holds it.
  saying: string;
  // What the model has reasoned in that same call, kept apart for the
  // reason the wire keeps the two increments apart: a page folds one
  // and reads the other.
  thinking: string;
}

// Mutable only through the store's own setter below; readers get the
// store, which Solid makes read-only.
export interface Belief {
  runs: Record<string, RunBelief>;
  halted: string[];
  // The last refusal a command came back with, for the page to show
  // once and the person to dismiss.
  refusal: AxError | null;
  city: string | null;
  // The last probe's answer: which endpoint, what it serves, what each
  // row stated, and where the call stopped. Held here rather than read
  // off the history's tail, where a long city would push it out of the
  // window.
  probed: Probed | null;
  // The tail of the process log, oldest first. A window rather than an
  // archive: a log is a diagnostic and not history, so the page keeps
  // what a person can still act on and drops the rest, which is also
  // what stops a city at the `wire` floor from filling this tab's
  // memory.
  logs: LogLine[];
}

// How many log lines the page keeps. Wide enough to hold the burst
// around one thing going wrong, narrow enough that a talkative city
// never becomes this tab's problem.
const LOG_WINDOW = 500;

function text(data: Record<string, unknown>, key: string): string | null {
  const held = data[key];
  return typeof held === "string" ? held : null;
}

// The one argument a person recognises a call by, in the order
// `channels::reading::SUBJECT_KEYS` prefers.
function subjectOf(data: Record<string, unknown>): string | null {
  const args = data.args;
  if (typeof args !== "object" || args === null) {
    return null;
  }
  const map: Record<string, unknown> = { ...args };
  for (const key of ["path", "addr", "program", "arm"]) {
    const named = text(map, key);
    if (named !== null) {
      return named;
    }
  }
  for (const value of Object.values(map)) {
    if (typeof value === "string") {
      return value;
    }
  }
  return null;
}

function fresh(run: RunId, record: EventRecord): RunBelief {
  return {
    run,
    addr: null,
    started: null,
    task: null,
    lastSeq: record.seq,
    lastKind: record.kind,
    doing: { kind: "thinking" },
    saying: "",
    thinking: "",
  };
}

// One record forward. Exhaustive over the kinds that move a run's
// posture; everything else only advances the position.
function fold(held: RunBelief, record: EventRecord): RunBelief {
  const data = record.data;
  const moved: RunBelief = { ...held, lastSeq: record.seq, lastKind: record.kind };
  switch (record.kind) {
    case "run_started":
      return {
        ...moved,
        addr: record.addr ?? null,
        started: record.t,
        task: text(data, "task"),
        doing: { kind: "thinking" },
      };
    case "model_called":
      return { ...moved, doing: { kind: "thinking" }, saying: "", thinking: "" };
    case "model_returned":
      return { ...moved, saying: "", thinking: "" };
    case "tool_called":
      return {
        ...moved,
        doing: {
          kind: "calling",
          tool: text(data, "name") ?? "tool",
          subject: subjectOf(data),
        },
      };
    case "tool_result":
      return { ...moved, doing: { kind: "thinking" } };
    case "approval_requested":
      return { ...moved, doing: { kind: "waiting" } };
    case "run_frozen":
      return {
        ...moved,
        saying: "",
        thinking: "",
        doing: { kind: "frozen", completion: text(data, "completion") },
      };
    // Every other kind only advances the position. Listed rather than
    // defaulted so a new kind is a decision here, not a silence.
    case "city_initialized":
    case "building_created":
    case "building_configured":
    case "run_forked":
    case "prompt_assembled":
    case "result_offloaded":
    case "gate_checked":
    case "gate_denied":
    case "checkpoint_committed":
    case "handoff_written":
    case "steer_received":
    case "cancel_received":
    case "watchdog_fired":
    case "budget_limit":
    case "log_truncated":
    case "signal_enqueued":
    case "signal_consumed":
    case "draft_held":
    case "draft_resolved":
    case "goal_registered":
    case "goal_conflict":
    case "arbitration_verdict":
    case "repair_started":
    case "repair_reused":
    case "worktree_opened":
    case "pr_opened":
    case "pr_merged":
    case "pr_rejected":
    case "roadmap_claimed":
    case "roadmap_finished":
    case "roadmap_released":
    case "roadmap_split":
    case "roadmap_blocked":
    case "approval_resolved":
    case "policy_created":
    case "policy_revoked":
    case "taint_promoted":
    case "cross_building_transfer":
    case "takeover_started":
    case "rollback_applied":
    case "city_halted":
    case "backpressure_shed":
    case "digest_invalidated":
    case "endpoint_attached":
    case "endpoint_lost":
    case "endpoint_probed":
    case "model_selected":
    case "provider_degraded":
    case "login_started":
    case "eval_run":
    case "asset_archived":
    case "credential_lent":
    case "secret_captured":
    case "secret_egress_blocked":
    case "file_discarded":
    case "discard_restored":
    case "autonomy_changed":
    case "pursuit_changed":
    case "governed_document_written":
      return moved;
  }
}

export function createBelief() {
  const [belief, setBelief] = createStore<Belief>({
    runs: {},
    halted: [],
    refusal: null,
    city: null,
    probed: null,
    logs: [],
  });

  // What a `city_view` answer says about runs this page never saw. A run
  // the page already follows keeps its own reading: the answer is a
  // position, and the page holds more than a position.
  function adoptCity(city: CityAnswer): void {
    setBelief(
      produce((draft) => {
        draft.halted = [...city.halted];
        for (const summary of city.runs) {
          const held = draft.runs[summary.run];
          if (held !== undefined && held.lastSeq >= summary.last_seq) {
            continue;
          }
          draft.runs[summary.run] = {
            run: summary.run,
            addr: summary.addr ?? held?.addr ?? null,
            started: summary.started ?? held?.started ?? null,
            task: held?.task ?? null,
            lastSeq: summary.last_seq,
            lastKind: summary.last_kind,
            doing: summary.frozen
              ? { kind: "frozen", completion: null }
              : (held?.doing ?? { kind: "thinking" }),
            saying: held?.saying ?? "",
            thinking: held?.thinking ?? "",
          };
        }
      }),
    );
  }

  function apply(record: EventRecord): void {
    setBelief(
      produce((draft) => {
        if (record.kind === "city_halted") {
          const scope = text(record.data, "scope");
          const state = text(record.data, "state");
          if (scope !== null) {
            const without = draft.halted.filter((held) => held !== scope);
            draft.halted = state === "halted" ? [...without, scope] : without;
          }
          return;
        }
        if (record.kind === "city_initialized") {
          draft.city = record.addr ?? null;
          return;
        }
        if (record.kind === "endpoint_probed") {
          const found = readProbed(record.data);
          if (found !== null) draft.probed = found;
          return;
        }
        // The nil run marks a record that belongs to the city itself.
        if (record.run === "00000000-0000-0000-0000-000000000000") {
          return;
        }
        const held = draft.runs[record.run] ?? fresh(record.run, record);
        if (held.lastSeq > record.seq) {
          return;
        }
        draft.runs[record.run] = fold(held, record);
      }),
    );
  }

  function say(delta: Delta): void {
    setBelief(
      produce((draft) => {
        const held = draft.runs[delta.run];
        if (held === undefined) {
          return;
        }
        if ("said" in delta.increment) {
          held.saying += delta.increment.said;
        } else {
          held.thinking += delta.increment.thought;
        }
      }),
    );
  }

  // One line of the process log, appended to the window. The oldest go
  // first, because what a person is reading a log for is what just
  // happened.
  function logged(line: LogLine): void {
    setBelief(
      produce((draft) => {
        draft.logs.push(line);
        if (draft.logs.length > LOG_WINDOW) {
          draft.logs.splice(0, draft.logs.length - LOG_WINDOW);
        }
      }),
    );
  }

  // A refusal is shown once; one kind also corrects the page: a steer
  // the city refuses because no run answers to that id means the run
  // this page still believes live is not, so it is frozen here rather
  // than left to swallow every further message as a steer.
  function refused(error: AxError | null): void {
    setBelief(
      produce((draft) => {
        draft.refusal = error;
        if (!error?.action.startsWith("steer")) {
          return;
        }
        const held = draft.runs[error.subject];
        if (held !== undefined && held.doing.kind !== "frozen") {
          draft.runs[error.subject] = { ...held, doing: { kind: "frozen", completion: null } };
        }
      }),
    );
  }

  // The name the welcome carried: a page that only hears what happens
  // next cannot otherwise know the name of a city raised last month.
  function named(city: string | null): void {
    setBelief("city", city);
  }

  return { belief, adoptCity, apply, say, logged, refused, named };
}

export type BeliefStore = ReturnType<typeof createBelief>;
