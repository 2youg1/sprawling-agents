// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";
import { writable } from "./store";
import type { Readable, Writable } from "./store";

import type { Key } from "./lang";
import { reachOf, reaches } from "./staleness";
import { Query } from "../wire";
import type {
  Address,
  Answer,
  AskId,
  AskOutcome,
  AxCode,
  AxError,
  EventRecord,
  Seq,
} from "../wire";

const PACE_MS = 250;

export const HELD_CAP = 128;

const PATIENCE_MS = 15_000;

export const QUERIES = {
  city: "city_view",
  approvals: "approval_queue",
  metrics: "metrics",
  cost: "cost_view",
  registry: "registry_view",
  discards: "discard_view",
  endpoints: "endpoint_view",
  knownHosts: "known_hosts",
  agentCatalog: "agent_catalog",
  devices: "devices",
  governance: "governance",
  doctor: "doctor",
  toolkits: "toolkits",
  release: "newest_release",
  preferences: "preferences",
  openProposals: "open_proposals",
  identity: "identity",
  automation: "automation",
  guide: "guide",
  shells: "shells",
  privacy: "privacy",
} as const satisfies Readonly<Record<string, Extract<Query, string>>>;

const SLOW_MS: Readonly<Partial<Record<string, number>>> = { [QUERIES.privacy]: 120_000 };

function patienceOf(query: Query): number {
  return SLOW_MS[nameOf(query)] ?? PATIENCE_MS;
}

export const COMMITS_PAGE = 40;

export function commitsQuery(building: Address | null, before: Seq | null): Query {
  return { commits: { building, before, limit: COMMITS_PAGE } };
}

export function githubLoginQuery(host: string | null): Query {
  return { github_login: host };
}

export type Reported = Omit<AxError, "recovery">;

interface Held {
  readonly value: Writable<Answer | undefined>;

  reported: boolean;
  stale: boolean;

  staleAt: number;

  asOf: Seq | null;
  inflight: boolean;
  watchers: number;
  timer: ReturnType<typeof setTimeout> | null;
}

interface Pending {
  readonly key: string;
  readonly query: Query;

  readonly due: number;
}

function minted(code: AxCode, action: string, subject: string): Reported {
  return { code, action, subject, nearby: [], retry: "no" };
}

export const keyOf = (query: Query): string => JSON.stringify(query);

const readKey = Schema.decodeOption(Schema.fromJsonString(Query));

export function askedIn(subject: string): string | null {
  return Option.match(readKey(subject), { onNone: () => null, onSome: nameOf });
}

function nameOf(query: Query): string {
  if (typeof query === "string") return query;
  return Object.keys(query)[0] ?? "";
}

export interface Asking {
  readonly ask: (query: Query) => Readable<Answer | undefined>;
  readonly refresh: (query: Query) => void;
  readonly answered: (askId: AskId, asOf: Seq, outcome: AskOutcome) => void;
  readonly invalidate: (record: EventRecord) => void;

  readonly reconnected: () => void;

  readonly resumed: () => void;
}

export function createAsking(
  send: (query: Query) => AskId | null,
  now: () => number,
  noticed: (phrase: Key, error: Reported) => void,
): Asking {
  const held = new Map<string, Held>();

  const pending = new Map<AskId, Pending>();

  const late = new Map<AskId, string>();
  const parsed = new Map<string, Query>();

  const byName = new Map<string, Set<string>>();

  let patience: { readonly timer: ReturnType<typeof setTimeout>; readonly due: number } | null =
    null;

  function watch(): void {
    const due = Math.min(...[...pending.values()].map((entry) => entry.due));
    if (due === Number.POSITIVE_INFINITY || (patience !== null && patience.due <= due)) return;
    if (patience !== null) clearTimeout(patience.timer);
    patience = { timer: setTimeout(sweep, Math.max(0, due - now())), due };
  }

  function sweep(): void {
    patience = null;
    const at = now();
    for (const [askId, entry] of pending) {
      if (entry.due > at) continue;
      pending.delete(askId);
      late.set(askId, entry.key);
      if (late.size > HELD_CAP) late.delete(late.keys().next().value ?? askId);
      const slot = held.get(entry.key);
      if (slot === undefined) continue;
      slot.inflight = false;
      slot.stale = true;
      if (!slot.reported) {
        slot.reported = true;
        noticed(
          "ask_late",
          minted("E_TIMEOUT", `answer the ${nameOf(entry.query)} this page asked`, entry.key),
        );
      }
      if (slot.watchers > 0) schedule(entry.key, entry.query, slot);
    }
    watch();
  }

  function dispatch(key: string, query: Query, slot: Held): void {
    if (slot.inflight) return;
    const askId = send(query);
    if (askId === null) {
      slot.stale = true;
      return;
    }
    slot.inflight = true;
    slot.stale = false;
    slot.staleAt = -1;
    pending.set(askId, { key, query, due: now() + patienceOf(query) });
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

  function watching(slot: Held): Readable<Answer | undefined> {
    return {
      subscribe(run) {
        slot.watchers += 1;
        const off = slot.value.subscribe(run);
        return () => {
          slot.watchers -= 1;
          off();
        };
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

    slot.staleAt = Number.POSITIVE_INFINITY;
    dispatch(key, query, slot);
  }

  function answered(askId: AskId, asOf: Seq, outcome: AskOutcome): void {
    const done = pending.get(askId);
    pending.delete(askId);
    const key = done?.key ?? late.get(askId);
    late.delete(askId);
    const slot = key === undefined ? undefined : held.get(key);
    if (key === undefined || slot === undefined) {
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
    if (patience !== null) clearTimeout(patience.timer);
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
