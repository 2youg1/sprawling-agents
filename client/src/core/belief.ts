// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this page believes about the city, folded forward from what the
// server pushes. It holds the little that has to move at the speed of
// the stream - which runs exist, what each is doing right now, the text
// a model is still saying - and nothing that a query answers better.
//
// **The reading of a record is `reading.ts`'s and the posture is
// `doing.ts`'s.** This file folds: one record into one run, one answer
// over the runs it names, one halt record against the list of shut
// scopes. A record whose payload this build cannot read still advances
// the position - that, the author and the time are readable - and the
// field it could not read comes back to the caller to report.

import { createStore, produce } from "solid-js/store";

import { readProbed } from "./probed";
import { PHASES, moves } from "./doing";
import type { Doing } from "./doing";
import { completionOf, haltOf, taskOf, toolCall } from "./reading";
import { sameScope } from "./scope";
import type { Probed } from "./probed";

import { CITY_RUN, Seq } from "../wire";
import type {
  Address,
  AxError,
  CityAnswer,
  Delta,
  EventRecord,
  HaltScope,
  LogLine,
  RunId,
  RunSummary,
  TimeMs,
} from "../wire";

export interface RunBelief {
  readonly run: RunId;
  readonly addr: Address | null;
  readonly started: TimeMs | null;
  readonly task: string | null;
  readonly lastSeq: Seq;
  readonly doing: Doing;
  // Heard from the stream and named by no answer yet: an answer folded
  // before the run began cannot name it, and that silence is not the
  // city saying the run is over.
  readonly local: boolean;
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
  halted: HaltScope[];
  // The ledger position the list of shut scopes is current to. Two
  // writers touch the list - an answer states the whole of it and a
  // record changes one scope - so without a position a gap folded back
  // in after the answer would undo it.
  haltedAt: Seq;
  // The last refusal a command came back with, for the page to show
  // once and the person to dismiss.
  refusal: AxError | null;
  // Every refusal this session has seen, newest last, bounded. The
  // dismissed one leaves the corner and stays here: a refusal a person
  // waved away is still the answer to what they asked.
  notices: Notice[];
  city: string | null;
  // The last probe's answer: which endpoint, what it serves, what each
  // row stated, and where the call stopped. Held here rather than read
  // off the history's tail, where a long city would push it out.
  probed: Probed | null;
  // The tail of the process log, oldest first. A window rather than an
  // archive: a log is a diagnostic and not history, so the page keeps
  // what a person can still act on, which is also what stops a city at
  // the `wire` floor from filling this tab's memory.
  logs: LogLine[];
}

// How many log lines the page keeps. Wide enough to hold the burst
// around one thing going wrong, narrow enough that a talkative city
// never becomes this tab's problem.
const LOG_WINDOW = 500;

// One refusal, kept after the corner has let go of it.
export interface Notice {
  readonly error: AxError;
  seen: boolean;
}

// How many refusals the bell keeps. Short on purpose: this is a list a
// person reads, not a record they audit - the ledger is where a city's
// history lives.
const NOTICE_WINDOW = 50;

// A run this page heard of and has no record of: a gap page starting
// after the beginning, or text that outran the run it belongs to. The
// position is before the first record (`Seq::FIRST`) and the phase is
// not yet a fact the stream has stated.
function unseen(run: RunId, at: Seq): RunBelief {
  return {
    run,
    addr: null,
    started: null,
    task: null,
    lastSeq: at,
    doing: { kind: "unknown" },
    local: true,
    saying: "",
    thinking: "",
  };
}

// What an answer says a frozen run's ending was. The completion is the
// record's, and an answer carries none, so a run this page never
// streamed is frozen with nothing to cite.
function frozen(held: RunBelief | undefined): Doing {
  return held?.doing.kind === "frozen" ? held.doing : PHASES.run_frozen;
}

// One `city_view` row read as a belief, keeping whatever the stream
// already knew that the row does not carry. The run page reads it too:
// a run reached by somebody's link was never streamed here, so the
// row is the only reading of that run this page has.
export function adopted(summary: RunSummary, held: RunBelief | undefined): RunBelief {
  // The page's own reading is the newer of the two, so it wins on
  // everything it knows; a field the stream never carried is still the
  // row's to state, and the answer settles nothing else but that.
  if (held !== undefined && held.lastSeq >= summary.last_seq) {
    const at = held.started ?? summary.started ?? null;
    return { ...held, addr: held.addr ?? summary.addr ?? null, started: at, local: false };
  }
  // The answer is the newer reading. Its `last_kind` states the phase
  // where the kind does; a kind that states none leaves what the page
  // already knew in place, and a run this page never saw keeps the one
  // thing it knows, which is that the run exists.
  const stated = moves(summary.last_kind) ? PHASES[summary.last_kind] : undefined;
  return {
    run: summary.run,
    addr: summary.addr ?? held?.addr ?? null,
    started: summary.started ?? held?.started ?? null,
    task: held?.task ?? null,
    lastSeq: summary.last_seq,
    doing: summary.frozen ? frozen(held) : (stated ?? held?.doing ?? { kind: "unknown" }),
    local: false,
    saying: held?.saying ?? "",
    thinking: held?.thinking ?? "",
  };
}

// One record forward, answering the run it produced and the name of the
// first field in it this build could not read.
function fold(held: RunBelief, record: EventRecord): [RunBelief, string | null] {
  const moved: RunBelief = { ...held, lastSeq: record.seq };
  switch (record.kind) {
    case "run_started": {
      const [task, bad] = taskOf(record);
      return [
        {
          ...moved,
          addr: record.addr ?? null,
          started: record.t,
          task,
          doing: PHASES.run_started,
        },
        bad,
      ];
    }
    case "model_called":
      return [{ ...moved, doing: PHASES.model_called, saying: "", thinking: "" }, null];
    case "model_returned":
      return [{ ...moved, saying: "", thinking: "" }, null];
    case "tool_called": {
      const [call, bad] = toolCall(record);
      return [
        { ...moved, doing: { kind: "calling", tool: call.name, subject: call.subject } },
        bad,
      ];
    }
    case "tool_result":
      return [{ ...moved, doing: PHASES.tool_result }, null];
    case "approval_requested":
      return [{ ...moved, doing: PHASES.approval_requested }, null];
    case "run_frozen": {
      const [completion, bad] = completionOf(record);
      return [
        {
          ...moved,
          saying: "",
          thinking: "",
          doing: { kind: "frozen", completion },
        },
        bad,
      ];
    }
    // Every other kind only advances the position. Listed rather than
    // defaulted so a new kind is a decision here, not a silence, and
    // grouped a family to a line so the list reads as one block: the
    // reader's question is which kinds move a run, not where one name
    // sits among sixty.
    case "city_initialized": case "city_halted": case "building_created":
    case "building_configured": case "run_forked": case "prompt_assembled":
    case "result_offloaded": case "log_truncated": case "gate_checked":
    case "gate_denied": case "approval_resolved": case "policy_created":
    case "policy_revoked": case "steer_received": case "cancel_received":
    case "watchdog_fired": case "budget_limit": case "backpressure_shed":
    case "signal_enqueued": case "signal_consumed": case "draft_held":
    case "draft_resolved": case "goal_registered": case "goal_conflict":
    case "arbitration_verdict": case "pursuit_changed": case "repair_started":
    case "repair_reused": case "worktree_opened": case "checkpoint_committed":
    case "handoff_written": case "pr_opened": case "pr_merged":
    case "pr_rejected": case "roadmap_claimed": case "roadmap_finished":
    case "roadmap_released": case "roadmap_split": case "roadmap_blocked":
    case "endpoint_attached": case "endpoint_lost": case "endpoint_probed":
    case "model_selected": case "provider_degraded": case "login_started":
    case "digest_invalidated": case "eval_run": case "asset_archived":
    case "toolkit_link_opened": case "credential_lent": case "secret_captured":
    case "secret_egress_blocked": case "file_discarded": case "discard_restored":
    case "autonomy_changed": case "taint_promoted": case "cross_building_transfer":
    case "takeover_started": case "rollback_applied": case "governed_document_written":
    case "embedding_called": case "rerank_called":
    case "adviser_asked": case "adviser_answered": case "adviser_fell_back":
      return [moved, null];
  }
}

export function createBelief() {
  const [belief, setBelief] = createStore<Belief>({
    runs: {},
    halted: [],
    haltedAt: Seq.make(0),
    refusal: null,
    notices: [],
    city: null,
    probed: null,
    logs: [],
  });

  // What a `city_view` answer says about runs this page never saw. A run
  // the page already follows keeps its own reading: the answer is a
  // position, and the page holds more than a position. A run the answer
  // does not name is dropped - a table that only grows ends a long
  // session in a tab nobody can use, and a run left behind by a restart
  // goes on telling the skyline that this city is busy.
  function adoptCity(city: CityAnswer): void {
    setBelief(
      produce((draft) => {
        // The answer states no ledger position of its own. The newest
        // position it does state - the furthest any run it lists has
        // been folded - is what it can claim by, so an answer whose runs
        // are all behind the list of shut scopes cannot take the page
        // back over an event already folded.
        const stated = city.runs.reduce(
          (far, run) => (run.last_seq > far ? run.last_seq : far),
          Seq.make(0),
        );
        if (stated >= draft.haltedAt) {
          draft.halted = [...city.halted];
          draft.haltedAt = stated;
        }
        for (const summary of city.runs) {
          draft.runs[summary.run] = adopted(summary, draft.runs[summary.run]);
        }
        const listed = new Set<string>(city.runs.map((summary) => summary.run));
        // The exemption is good for one answer: by the next one the
        // city has had its chance to list the run, which is what
        // evicts a run a restarted city no longer has.
        const kept: Record<string, RunBelief> = {};
        for (const [run, held] of Object.entries(draft.runs)) {
          if (listed.has(run)) kept[run] = held;
          else if (held.local) kept[run] = { ...held, local: false };
        }
        draft.runs = kept;
      }),
    );
  }

  // One `city_halted` record, which writes the list only when it is
  // newer than what the list is current to. The two words decide between
  // stopping and starting, so a record this build cannot read changes
  // nothing rather than being read as a release.
  function halted(record: EventRecord): string | null {
    const [held, bad] = haltOf(record);
    const scope = held.scope;
    const state = held.state;
    if (scope === null || state === null) return bad;
    if (record.seq > belief.haltedAt) {
      setBelief(
        produce((draft) => {
          const without = draft.halted.filter((each) => !sameScope(each, scope));
          draft.halted = state === "halted" ? [...without, scope] : without;
          draft.haltedAt = record.seq;
        }),
      );
    }
    return null;
  }

  // One record in, answering the name of the first field in it this
  // build could not read - `null` when it read the whole record.
  function apply(record: EventRecord): string | null {
    if (record.kind === "city_halted") {
      return halted(record);
    }
    if (record.kind === "city_initialized") {
      setBelief("city", record.addr ?? null);
      return null;
    }
    if (record.kind === "endpoint_probed") {
      const found = readProbed(record.data);
      if (found !== null) setBelief("probed", found);
      return null;
    }
    if (record.run === CITY_RUN) {
      return null;
    }
    const held = belief.runs[record.run] ?? unseen(record.run, record.seq);
    if (held.lastSeq > record.seq) {
      return null;
    }
    const [next, bad] = fold(held, record);
    setBelief("runs", record.run, next);
    return bad;
  }

  // One piece of what the model is producing. A page that joins in the
  // middle of a call hears the model before it hears the run, and those
  // words are held on a belief of their own rather than dropped: the
  // run's records fill in the address, the task and the phase as they
  // arrive, and the next answer that does not list the run takes it
  // away, because every run born here is `local`.
  function say(delta: Delta): void {
    setBelief(
      produce((draft) => {
        const held = draft.runs[delta.run] ?? unseen(delta.run, Seq.make(0));
        if ("said" in delta.increment) {
          held.saying += delta.increment.said;
        } else {
          held.thinking += delta.increment.thought;
        }
        draft.runs[delta.run] = held;
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
  //
  // **The words decide this, and the code cannot.** `Steer` and `Cancel`
  // answer the same `E_INVALID_ARGS` when no run answers to the id
  // (`assembly::commanding::routing`), so a client matching the code
  // would freeze a run over a refused cancel; the action sentence is the
  // only discriminator the server states.
  function refused(error: AxError | null): void {
    setBelief(
      produce((draft) => {
        draft.refusal = error;
        if (error !== null) {
          draft.notices.push({ error, seen: false });
          if (draft.notices.length > NOTICE_WINDOW) {
            draft.notices.splice(0, draft.notices.length - NOTICE_WINDOW);
          }
        }
        if (!error?.action.startsWith("steer")) {
          return;
        }
        const held = draft.runs[error.subject];
        if (held !== undefined && held.doing.kind !== "frozen") {
          draft.runs[error.subject] = { ...held, doing: PHASES.run_frozen };
        }
      }),
    );
  }

  // The name the welcome carried: a page that only hears what happens
  // next cannot otherwise know the name of a city raised last month.
  function named(city: string | null): void {
    setBelief("city", city);
  }

  // Reading the bell is what marks it read: an unread count that
  // survived the panel being open would be a number nobody can clear.
  function noticesSeen(): void {
    setBelief(
      produce((draft) => {
        for (const notice of draft.notices) {
          notice.seen = true;
        }
      }),
    );
  }

  return { belief, adoptCity, apply, say, logged, refused, named, noticesSeen };
}

export type BeliefStore = ReturnType<typeof createBelief>;
