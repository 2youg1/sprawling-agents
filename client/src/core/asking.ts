// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this page has asked, what it keeps, and when it asks again.
//
// A query is asked once and its answer kept; a page draws what it has
// while a fresh answer is on its way (stale while revalidating), so a
// route change never waits on the socket. An event that could have
// changed an answer marks it stale, and a stale answer somebody is
// looking at is asked again at most once per `PACE_MS` - a burst of a
// hundred records becomes one question, not a hundred.
//
// Every question goes out under an `AskId` the socket mints, and its
// answer comes back under the same id with the ledger position it was
// read at, so the pairing is exact and an event that position already
// holds marks nothing stale. An answer that matches nothing, and a
// question nothing answers, are reported.

import { writable } from "svelte/store";
import type { Readable, Writable } from "svelte/store";

import type { Key } from "./lang";
import { reachOf, reaches } from "./staleness";
import type { Address, Answer, AskId, AskOutcome, AxCode, AxError, EventRecord, Query, Seq } from "../wire";

const PACE_MS = 250;

// How many answers a page keeps. A page draws a few dozen at once; the
// rest are what a person may come back to, and past this count the one
// used least recently and watched by nobody is dropped, so a tab left
// open for a week holds what it holds on the first day.
export const HELD_CAP = 128;

// How long one question may go unanswered before the page stops
// drawing a skeleton over it. One home for the deadline: the sweep
// judges every question against it, and no phrase names a number.
const PATIENCE_MS = 15_000;

// Every question that needs nothing said after its name, under the
// field its answer arrives in, so a question is spelled once for the
// whole client. The generated `Query` admits
// only names this build can ask, so a misspelling fails to compile.
export const QUERIES = {
  city: "city_view",
  approvals: "approval_queue",
  metrics: "metrics",
  cost: "cost_view",
  registry: "registry_view",
  discards: "discard_view",
  endpoints: "endpoint_view",
  governance: "governance",
  doctor: "doctor",
  toolkits: "toolkits",
  release: "release",
  preferences: "preferences",
} as const satisfies Readonly<Record<string, Extract<Query, string>>>;

// One page of a building's commits. The answer carries the building
// and the bound back but not the page size, so every page asks for the
// same size and the answer is matched by the other two.
export const COMMITS_PAGE = 40;

// The one spelling of the commits question, so the key a page asks
// under and the key its answer is filed under cannot differ.
export function commitsQuery(building: Address | null, before: Seq | null): Query {
  return { commits: { building, before, limit: COMMITS_PAGE } };
}

// Everything a refusal states but the sentence a person reads.
export type Reported = Omit<AxError, "recovery">;

interface Held {
  readonly value: Writable<Answer | undefined>;
  // Whether this question has already been reported unanswered. One
  // report per stretch of silence: a person told every fifteen seconds
  // that the city is not answering learns nothing after the first time.
  reported: boolean;
  stale: boolean;
  // The newest record that marked this answer stale while its question
  // was out: an answer read past it already holds it.
  staleAt: number;
  // The first seq the held answer does not reflect; a record before it
  // is already in the answer.
  asOf: Seq | null;
  inflight: boolean;
  watchers: number;
  timer: ReturnType<typeof setTimeout> | null;
}

interface Pending {
  readonly key: string;
  readonly query: Query;
  // When it went out, by the clock handed to `createAsking`.
  readonly sentAt: number;
}

// A refusal this page minted about its own asking. The recovery is a
// sentence for a person and therefore `lang.json`'s: the reporter names
// the phrase, the seam that knows the language says it.
function minted(code: AxCode, action: string, subject: string): Reported {
  return { code, action, subject, nearby: [], retry: "no" };
}

export const keyOf = (query: Query): string => JSON.stringify(query);

// The wire name of a query, as `Query::name` spells it.
function nameOf(query: Query): string {
  if (typeof query === "string") return query;
  return Object.keys(query)[0] ?? "";
}

export interface Asking {
  // The answer to one question, as a store: `undefined` until the first
  // answer lands. Subscribing counts as watching; a watched answer is
  // refreshed when stale, an unwatched one waits until somebody looks.
  readonly ask: (query: Query) => Readable<Answer | undefined>;
  readonly refresh: (query: Query) => void;
  readonly answered: (askId: AskId, asOf: Seq, outcome: AskOutcome) => void;
  readonly invalidate: (record: EventRecord) => void;
  // Everything is stale after a reconnect the page cannot resume: it
  // missed whatever happened while the socket was down.
  readonly reconnected: () => void;
  // A reconnect whose missed records are being fetched: those records
  // invalidate what they touch, so only the questions the dead socket
  // took with it are asked again.
  readonly resumed: () => void;
}

// The clock and the ear are handed in: a deadline a test cannot drive
// is a rule nobody has checked, and this module phrases nothing for a
// person - it names the `lang.json` phrase and the seam that knows the
// person's language says it.
// `send` returns the id the question went out under, or null when the
// link is not live.
export function createAsking(
  send: (query: Query) => AskId | null,
  now: () => number,
  noticed: (phrase: Key, error: Reported) => void,
): Asking {
  const held = new Map<string, Held>();
  // Insertion order is send order, so the first entry is the oldest.
  const pending = new Map<AskId, Pending>();
  // Questions whose patience ran out, kept so a late answer still lands
  // on the slot that asked; bounded like the held answers.
  const late = new Map<AskId, string>();
  const parsed = new Map<string, Query>();
  // The keys held under each question name: a record is judged once per
  // name rather than once per held answer.
  const byName = new Map<string, Set<string>>();
  // One patience timer for the whole queue: the clock says which
  // questions are late, so a timer each would only be one more thing
  // to cancel.
  let patience: ReturnType<typeof setTimeout> | null = null;

  function watch(): void {
    const oldest = pending.values().next().value;
    if (patience !== null || oldest === undefined) return;
    patience = setTimeout(sweep, Math.max(0, oldest.sentAt + PATIENCE_MS - now()));
  }

  // Every question whose patience has run out: out of the queue, and
  // its slot stale rather than empty, so an answer already on screen
  // stays there while the page asks again. What the person gets is
  // one refusal in the corner and in the bell, because a page that
  // waits for ever in silence is the defect this is here for.
  function sweep(): void {
    patience = null;
    const deadline = now() - PATIENCE_MS;
    for (const [askId, entry] of pending) {
      if (entry.sentAt > deadline) break;
      pending.delete(askId);
      late.set(askId, entry.key);
      if (late.size > HELD_CAP) late.delete(late.keys().next().value ?? askId);
      const slot = held.get(entry.key);
      if (slot === undefined) continue;
      slot.inflight = false;
      slot.stale = true;
      if (!slot.reported) {
        slot.reported = true;
        noticed("ask_late", minted("E_TIMEOUT", `answer the ${nameOf(entry.query)} this page asked`, entry.key));
      }
      if (slot.watchers > 0) schedule(entry.key, entry.query, slot);
    }
    watch();
  }

  function dispatch(key: string, query: Query, slot: Held): void {
    if (slot.inflight) return;
    const askId = send(query);
    if (askId === null) {
      // Not live: it is asked again when the link comes back.
      slot.stale = true;
      return;
    }
    slot.inflight = true;
    slot.stale = false;
    slot.staleAt = -1;
    pending.set(askId, { key, query, sentAt: now() });
    watch();
  }

  function schedule(key: string, query: Query, slot: Held): void {
    if (slot.timer !== null) return;
    slot.timer = setTimeout(() => {
      slot.timer = null;
      if (slot.stale && slot.watchers > 0) dispatch(key, query, slot);
    }, PACE_MS);
  }

  function slotFor(query: Query): [string, Held] {
    const key = keyOf(query);
    const found = held.get(key);
    if (found !== undefined) {
      // Re-inserted, so the map's order is the order of last use.
      held.delete(key);
      held.set(key, found);
      return [key, found];
    }
    const slot: Held = {
      value: writable<Answer | undefined>(undefined),
      reported: false,
      stale: true,
      staleAt: -1,
      asOf: null,
      inflight: false,
      watchers: 0,
      timer: null,
    };
    held.set(key, slot);
    parsed.set(key, query);
    const name = nameOf(query);
    const named = byName.get(name) ?? new Set<string>();
    byName.set(name, named.add(key));
    if (held.size > HELD_CAP) dropLeastRecent();
    return [key, slot];
  }

  // A slot somebody watches, or one still waiting on its answer or its
  // pace timer, stays: dropping it would strand a subscriber or leave an
  // answer with nowhere to land.
  function dropLeastRecent(): void {
    for (const [key, slot] of held) {
      if (slot.watchers > 0 || slot.inflight || slot.timer !== null) continue;
      held.delete(key);
      const query = parsed.get(key);
      parsed.delete(key);
      if (query !== undefined) byName.get(nameOf(query))?.delete(key);
      return;
    }
  }

  // The public face of one slot: subscribing is what makes an answer
  // watched, so an answer nobody looks at holds nothing open.
  function watching(slot: Held): Readable<Answer | undefined> {
    return {
      subscribe(run) {
        slot.watchers += 1;
        const off = slot.value.subscribe(run);
        return () => { slot.watchers -= 1; off(); };
      },
    };
  }

  function ask(query: Query): Readable<Answer | undefined> {
    const [key, slot] = slotFor(query);
    if (slot.stale) dispatch(key, query, slot);
    return watching(slot);
  }

  function refresh(query: Query): void {
    const [key, slot] = slotFor(query);
    slot.stale = true;
    // Asked for by the person, so no ledger position can satisfy it.
    slot.staleAt = Number.POSITIVE_INFINITY;
    dispatch(key, query, slot);
  }

  // A question's outcome, under the id it went out with. A refusal
  // leaves the slot stale for the next watcher to ask again; the
  // socket has already put the refusal in front of the person.
  function answered(askId: AskId, asOf: Seq, outcome: AskOutcome): void {
    const done = pending.get(askId);
    pending.delete(askId);
    const key = done?.key ?? late.get(askId);
    late.delete(askId);
    const slot = key === undefined ? undefined : held.get(key);
    if (key === undefined || slot === undefined) {
      // Nothing here waits on it, and nothing is said: no question waits
      // on it, so there is nothing a person could do, and a page and a
      // city from one build are not "versions that differ". A question
      // that does wait is still pending, where the sweep reports it once
      // its patience runs out.
      return;
    }
    if (done !== undefined) slot.inflight = false;
    if ("refusal" in outcome) {
      slot.stale = true;
      return;
    }
    slot.reported = false;
    slot.asOf = asOf;
    if (slot.stale && slot.staleAt < asOf) slot.stale = false;
    slot.value.set(outcome.answer);
    const query = parsed.get(key);
    if (slot.stale && slot.watchers > 0 && query !== undefined) schedule(key, query, slot);
  }

  function invalidate(record: EventRecord): void {
    for (const [name, keys] of byName) {
      const reach = reachOf(name, record.kind);
      if (reach === "none") continue;
      for (const key of keys) {
        const slot = held.get(key);
        const query = parsed.get(key);
        if (slot === undefined || query === undefined || !reaches(reach, key, record.run)) continue;
        if (slot.asOf !== null && record.seq < slot.asOf) continue;
        slot.stale = true;
        slot.staleAt = Math.max(slot.staleAt, record.seq);
        if (slot.watchers > 0) schedule(key, query, slot);
      }
    }
  }

  function reconnected(): void {
    for (const slot of held.values()) slot.stale = true;
    resumed();
  }

  function resumed(): void {
    pending.clear();
    late.clear();
    if (patience !== null) clearTimeout(patience);
    patience = null;
    for (const [key, slot] of held) {
      if (slot.inflight) slot.stale = true;
      slot.inflight = false;
      slot.reported = false;
      const query = parsed.get(key);
      if (slot.stale && query !== undefined && slot.watchers > 0) dispatch(key, query, slot);
    }
  }

  return { ask, refresh, answered, invalidate, reconnected, resumed };
}
