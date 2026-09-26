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

import { Effect } from "effect";
import { writable } from "svelte/store";
import type { Readable } from "svelte/store";

import { readProbed } from "./probed";
import { PHASES } from "./doing";
import { completionOf, haltOf, sessionStart, taskOf, toolCall } from "./reading";
import { sameScope } from "./scope";

import { CITY_RUN, Seq, TimeMs } from "../wire";
import type { AxError, CityAnswer, Delta, EventRecord, LogLine, RunId } from "../wire";

import type { Belief, RunBelief } from "./belief/shape";
import { LOG_WINDOW, merged } from "./belief/shape";
import { runTable } from "./belief/runs.svelte";
import { adopted } from "./belief/adopted";
import { livened, liveOf } from "./belief/live";
import { roomed, roomsOf } from "./belief/rooms";
import { cancelledOf, recounted } from "./belief/cancelled";
export type { Belief, Notice, RunBelief } from "./belief/shape";

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
    case "session_opened":
    case "city_initialized": case "city_halted": case "building_created":
    case "building_configured": case "run_forked": case "prompt_assembled":
    case "prompt_shape_compared":
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
    case "governed_document_written":
    case "spine_document_written": case "rules_changed":
    case "embedding_called": case "rerank_called":
    case "adviser_asked": case "adviser_answered": case "adviser_fell_back":
      return [moved, null];
  }
}

// The store and its folds, as one door. `now` is handed in - a notice
// is stamped when it first arrived, and a clock a test cannot move is a
// stamp nobody has checked.
export interface BeliefStore {
  readonly belief: Readable<Belief>;
  readonly adoptCity: (city: CityAnswer) => void;
  readonly apply: (record: EventRecord) => string | null;
  readonly say: (delta: Delta) => void;
  readonly logged: (line: LogLine) => void;
  readonly refused: (error: AxError | null) => void;
  readonly named: (city: string | null) => void;
  readonly noticesSeen: () => void;
  // Drops every fold: what the page held came from another ledger.
  readonly forget: () => void;
  // Runs `folds` and tells subscribers once, after the last of them:
  // a burst of records between two paints is one update and one paint.
  // Batches nest; only the outermost one publishes.
  readonly batch: (folds: () => void) => void;
}

export function createBelief(now: () => number): BeliefStore {
  // The belief as the folds have left it, and the store that tells
  // subscribers about it. They differ only inside a batch; reading the
  // local rather than the store keeps a fold from subscribing and
  // unsubscribing once per record.
  const empty = (): Belief => ({
    runs: runTable({}),
    live: [],
    rooms: new Map(),
    cancelled: 0,
    halted: [],
    haltedAt: Seq.make(0),
    refusal: null,
    notices: [],
    city: null,
    sessions: {},
    probed: null,
    logs: [],
  });
  let current: Belief = empty();
  const store = writable<Belief>(current);
  let depth = 0;

  function written(next: Belief): void {
    current = next;
    if (depth === 0) store.set(next);
  }

  // A fold that fails still closes its batch: a depth left above zero
  // would stop every later write from reaching a subscriber.
  function batch(folds: () => void): void {
    const before = current;
    depth += 1;
    Effect.runSync(
      Effect.sync(folds).pipe(
        Effect.ensuring(
          Effect.sync(() => {
            depth -= 1;
            if (depth === 0 && current !== before) store.set(current);
          }),
        ),
      ),
    );
  }

  // What a `city_view` answer says about runs this page never saw. A run
  // the page already follows keeps its own reading: the answer is a
  // position, and the page holds more than a position. A run the answer
  // does not name is dropped - a table that only grows ends a long
  // session in a tab nobody can use, and a run left behind by a restart
  // goes on telling the skyline that this city is busy.
  function adoptCity(city: CityAnswer): void {
    const held = current;
    // The answer states no ledger position of its own. The newest
    // position it does state - the furthest any run it lists has been
    // folded - is what it can claim by, so an answer whose runs are all
    // behind the list of shut scopes cannot take the page back over an
    // event already folded.
    const stated = city.runs.reduce(
      (far, run) => (run.last_seq > far ? run.last_seq : far),
      Seq.make(0),
    );
    const runs: Record<string, RunBelief> = {};
    const listed = new Set<string>();
    for (const summary of city.runs) {
      listed.add(summary.run);
      runs[summary.run] = adopted(summary, held.runs[summary.run]);
    }
    // The exemption for a run only the stream introduced is good for one
    // answer: by the next one the city has had its chance to list the
    // run, which is what evicts a run a restarted city no longer has.
    for (const [run, was] of Object.entries(held.runs)) {
      if (listed.has(run)) continue;
      if (was.local) runs[run] = { ...was, local: false };
    }
    const table = runTable(runs);
    written({
      ...held,
      runs: table,
      live: liveOf(table),
      rooms: roomsOf(table),
      cancelled: cancelledOf(table),
      halted: stated >= held.haltedAt ? [...city.halted] : held.halted,
      haltedAt: stated >= held.haltedAt ? stated : held.haltedAt,
    });
  }

  // One `city_halted` record, which writes the list only when it is
  // newer than what the list is current to. The two words decide between
  // stopping and starting, so a record this build cannot read changes
  // nothing rather than being read as a release.
  function halted(record: EventRecord): string | null {
    const [stated, bad] = haltOf(record);
    const scope = stated.scope;
    const word = stated.state;
    if (scope === null || word === null) return bad;
    const held = current;
    if (record.seq > held.haltedAt) {
      const without = held.halted.filter((each) => !sameScope(each, scope));
      written({
        ...held,
        halted: word === "halted" ? [...without, scope] : without,
        haltedAt: record.seq,
      });
    }
    return null;
  }

  // One record in, answering the name of the first field in it this
  // build could not read - `null` when it read the whole record.
  function apply(record: EventRecord): string | null {
    if (record.kind === "city_halted") {
      return halted(record);
    }
    const held = current;
    if (record.kind === "city_initialized") {
      written({ ...held, city: record.addr ?? null });
      return null;
    }
    const start = sessionStart(record);
    if (start !== null) {
      // Written only forwards: a page that reloads folds an older range
      // after a newer one, and the newest start is what a stretch begins at.
      if (start.seq > (held.sessions[start.addr] ?? Seq.make(0))) {
        written({ ...held, sessions: { ...held.sessions, [start.addr]: start.seq } });
      }
      return null;
    }
    if (record.kind === "endpoint_probed") {
      const found = readProbed(record.data);
      if (found !== null) written({ ...held, probed: found });
      return null;
    }
    if (record.run === CITY_RUN) {
      return null;
    }
    const run = held.runs[record.run] ?? unseen(record.run, record.seq);
    if (run.lastSeq > record.seq) {
      return null;
    }
    const [next, bad] = fold(run, record);
    folded(held, record.run, next);
    return bad;
  }

  // One run's new reading, written into the table in place. Copying the
  // table per record made one token cost a pass over every run the city
  // holds; the table is the store's own, so the new top-level belief is
  // what tells a subscriber that something moved, and every run it holds
  // is a fresh object whenever its reading changed.
  //
  // The working runs move with it: the index holds the table's own
  // reading of the run, so the words written into it in place reach a
  // reader of the index too.
  function folded(held: Belief, run: RunId, next: RunBelief): void {
    const was = held.runs[run];
    const rooms = roomed(held, was, next);
    const cancelled = recounted(held.cancelled, was, next);
    held.runs[run] = next;
    written({ ...held, live: livened(held.live, held.runs[run] ?? next), rooms, cancelled });
  }

  // One piece of what the model is producing. A page that joins in the
  // middle of a call hears the model before it hears the run, and those
  // words are held on a belief of their own rather than dropped: the
  // run's records fill in the address, the task and the phase as they
  // arrive, and the next answer that does not list the run takes it
  // away, because every run born here is `local`.
  //
  // A run already held takes the words in place: its field is `$state`,
  // so the readers of that run's words redraw and the belief is not
  // republished. Only a run the words introduce changes the table's
  // shape, and that is published.
  function say(delta: Delta): void {
    const held = current;
    const run = held.runs[delta.run];
    if (run === undefined) {
      const born = unseen(delta.run, Seq.make(0));
      folded(
        held,
        delta.run,
        "said" in delta.increment
          ? { ...born, saying: delta.increment.said }
          : { ...born, thinking: delta.increment.thought },
      );
      return;
    }
    if ("said" in delta.increment) run.saying += delta.increment.said;
    else run.thinking += delta.increment.thought;
  }

  // One line of the process log, appended to the window. The oldest go
  // first, because what a person is reading a log for is what just
  // happened.
  function logged(line: LogLine): void {
    const held = current;
    const logs = [...held.logs, line];
    if (logs.length > LOG_WINDOW) {
      logs.splice(0, logs.length - LOG_WINDOW);
    }
    written({ ...held, logs });
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
    const held = current;
    if (error === null) {
      written({ ...held, refusal: null });
      return;
    }
    const notices = merged(held.notices, error, TimeMs.make(now()));
    const run = error.action.startsWith("steer") ? held.runs[error.subject] : undefined;
    batch(() => {
      if (run !== undefined && run.doing.kind !== "frozen") {
        folded(held, run.run, { ...run, doing: PHASES.run_frozen });
      }
      written({ ...current, refusal: error, notices });
    });
  }

  // The name the welcome carried: a page that only hears what happens
  // next cannot otherwise know the name of a city raised last month.
  function named(city: string | null): void {
    written({ ...current, city });
  }

  // Reading the bell is what marks it read: an unread count that
  // survived the panel being open would be a number nobody can clear.
  function noticesSeen(): void {
    const held = current;
    written({ ...held, notices: held.notices.map((each) => ({ ...each, seen: true })) });
  }

  function forget(): void {
    written(empty());
  }

  return { belief: store, adoptCity, apply, say, logged, refused, named, noticesSeen, forget, batch };
}
