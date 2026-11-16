// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Effect } from "effect";
import { writable } from "svelte/store";
import type { Readable } from "svelte/store";

import { readProbed } from "./probed";
import { PHASES } from "./doing";
import { haltOf, policyChange, sessionStart } from "./reading";
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
import { fold, unseen } from "./belief/fold";
export type { Belief, Notice, RunBelief } from "./belief/shape";

export interface BeliefStore {
  readonly belief: Readable<Belief>;
  readonly adoptCity: (city: CityAnswer) => void;
  readonly apply: (record: EventRecord) => string | null;
  readonly say: (delta: Delta) => void;
  readonly logged: (line: LogLine) => void;
  readonly refused: (error: AxError | null) => void;
  readonly named: (city: string | null) => void;
  readonly noticesSeen: () => void;
  readonly forget: () => void;
  readonly batch: (folds: () => void) => void;
}

export function createBelief(now: () => number): BeliefStore {
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
    policies: {},
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

  function adoptCity(city: CityAnswer): void {
    const held = current;
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
      if (start.seq > (held.sessions[start.addr] ?? Seq.make(0))) {
        written({ ...held, sessions: { ...held.sessions, [start.addr]: start.seq } });
      }
      return null;
    }
    const changed = policyChange(record);
    if (changed !== null && changed.seq > (held.policies[changed.addr]?.seq ?? Seq.make(0))) {
      written({ ...held, policies: { ...held.policies, [changed.addr]: changed } });
    }
    if (record.kind === "endpoint_probed") {
      const found = readProbed(record.data);
      if (found !== null) written({ ...held, probed: found });
      return null;
    }
    if (record.run === CITY_RUN) {
      return null;
    }
    return foldRun(record);
  }

  function foldRun(record: EventRecord): string | null {
    const held = current;
    const run = held.runs[record.run] ?? unseen(record.run, record.seq);
    if (run.lastSeq > record.seq) {
      return null;
    }
    const [next, bad] = fold(run, record);
    folded(held, record.run, next);
    return bad;
  }

  function folded(held: Belief, run: RunId, next: RunBelief): void {
    const was = held.runs[run];
    const rooms = roomed(held, was, next);
    const cancelled = recounted(held.cancelled, was, next);
    held.runs[run] = next;
    written({ ...held, live: livened(held.live, held.runs[run] ?? next), rooms, cancelled });
  }

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

  function logged(line: LogLine): void {
    const held = current;
    const logs = [...held.logs, line];
    if (logs.length > LOG_WINDOW) {
      logs.splice(0, logs.length - LOG_WINDOW);
    }
    written({ ...held, logs });
  }

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

  function named(city: string | null): void {
    written({ ...current, city });
  }

  function noticesSeen(): void {
    const held = current;
    written({ ...held, notices: held.notices.map((each) => ({ ...each, seen: true })) });
  }

  function forget(): void {
    written(empty());
  }

  return {
    belief: store,
    adoptCity,
    apply,
    say,
    logged,
    refused,
    named,
    noticesSeen,
    forget,
    batch,
  };
}
