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
// The wire carries no request id, so an answer is matched to the
// question by its own content where the answer names it (a run, an
// address, a node) and by arrival order where it cannot. An answer
// that matches nothing, and a question nothing answers, are reported
// rather than waited on for ever.

import { createSignal, getOwner, onCleanup } from "solid-js";
import type { Accessor } from "solid-js";

import type { Key } from "./lang";
import type { Address, Answer, AxCode, AxError, EventKind, EventRecord, Query, Seq } from "../wire";

const PACE_MS = 250;

// How long one question may go unanswered before the page stops
// drawing a skeleton over it. One home for the deadline: the sweep
// judges every question against it, and no phrase names a number.
const PATIENCE_MS = 15_000;

// Every question that needs nothing said after its name, under the
// field its answer arrives in. Two duties, one table: a view asks
// through it, so a question is spelled once for the whole client, and
// `keyOfAnswer` walks it, so the pairing between an answer and the
// question it settles cannot lose a row. The generated `Query` admits
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
  readonly value: Accessor<Answer | undefined>;
  readonly set: (answer: Answer) => void;
  // Whether this question has already been reported unanswered. One
  // report per stretch of silence: the page goes on asking, and a
  // person told every fifteen seconds that the city is not answering
  // learns nothing after the first time.
  reported: boolean;
  stale: boolean;
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

// A refusal this page minted about its own asking, in the shape the
// city writes its own - except for the recovery, which is a sentence
// for a person and therefore `lang.json`'s to hold: the reporter names
// the phrase, and the seam that knows the person's language says it.
function minted(code: AxCode, action: string, subject: string): Reported {
  return { code, action, subject, nearby: [], retriable: false };
}

export const keyOf = (query: Query): string => JSON.stringify(query);

// The wire name of a query, as `Query::name` spells it.
function nameOf(query: Query): string {
  if (typeof query === "string") return query;
  return Object.keys(query)[0] ?? "";
}

// The key an answer would have been asked under, when the answer says.
function keyOfAnswer(answer: Answer): string | null {
  for (const [field, query] of Object.entries(QUERIES)) {
    if (field in answer) return keyOf(query);
  }
  if ("building" in answer) return keyOf({ building_view: { addr: answer.building.addr } });
  if ("inbox" in answer) return keyOf({ inbox_view: { addr: answer.inbox.addr } });
  if ("archive" in answer) return keyOf({ archive_search: { needle: answer.archive.needle } });
  if ("rounds" in answer) return keyOf({ rounds: { run: answer.rounds.run } });
  if ("evidence" in answer) return keyOf({ evidence: { run: answer.evidence.run } });
  if ("cost_of" in answer) return keyOf({ cost_of: { node: answer.cost_of.node } });
  if ("listing" in answer) return keyOf({ listing: { at: answer.listing.at ?? null } });
  if ("document" in answer) return keyOf({ document: { at: answer.document.at } });
  if ("prefix" in answer) return keyOf({ prefix: { run: answer.prefix.run } });
  if ("content" in answer) return keyOf({ content: { locator: answer.content.locator } });
  if ("skills" in answer) return keyOf({ skills: { building: answer.skills.building } });
  if ("git_status" in answer) return keyOf({ git_status: { building: answer.git_status.building } });
  if ("commit" in answer) return keyOf({ commit: { oid: answer.commit.oid } });
  if ("mcp_health" in answer) return keyOf({ mcp_health: { addr: answer.mcp_health.addr } });
  if ("hunks" in answer) {
    const { oid_a, oid_b, path } = answer.hunks;
    return keyOf({ hunks: { oid_a, oid_b, path } });
  }
  if ("changes" in answer) {
    const { base, head } = answer.changes;
    return keyOf({ changes: { base, head: head ?? null } });
  }
  if ("commits" in answer) return keyOf(commitsQuery(answer.commits.building ?? null, answer.commits.before ?? null));
  return null;
}

// Which pending question an answer with no name of its own belongs to:
// the oldest one of a kind that could have produced it.
function kindsOf(answer: Answer): readonly string[] {
  if ("history" in answer) return ["history", "run_history"];
  if ("run" in answer) return ["run_view"];
  if ("unavailable" in answer) {
    const head = answer.unavailable.query.split("(")[0] ?? "";
    // `BuildingView` -> `building_view`.
    const snake = head.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
    return [snake];
  }
  return [];
}

// What moves a provider's standing, a decision, and a building's
// papers. Named rather than written out at the arm that reads them: a
// set has a name a reader can hold, and the arm then states which
// question it belongs to and nothing else.
const PROVIDER_MOVED: ReadonlySet<EventKind> = new Set<EventKind>([
  "endpoint_attached", "endpoint_lost", "endpoint_probed",
  "model_selected", "login_started", "provider_degraded",
]);

const GOVERNANCE_MOVED: ReadonlySet<EventKind> = new Set<EventKind>([
  "approval_resolved", "autonomy_changed", "policy_created", "policy_revoked",
]);

const BUILDING_MOVED: ReadonlySet<EventKind> = new Set<EventKind>([
  "building_created", "building_configured", "roadmap_claimed", "roadmap_finished",
  "roadmap_released", "roadmap_split", "roadmap_blocked", "pursuit_changed",
  "checkpoint_committed", "handoff_written", "run_started", "run_frozen",
  "pr_merged", "asset_archived", "governed_document_written",
]);

// Which held answers one record makes stale.
function staleBy(record: EventRecord, key: string, query: Query): boolean {
  const name = nameOf(query);
  const run = record.run;
  const kind = record.kind;
  switch (name) {
    case "city_view": case "metrics":
      return true;
    case "rounds": case "evidence": case "run_view": case "run_history":
      return key.includes(run);
    case "history":
      return true;
    case "approval_queue":
      return kind === "approval_requested" || kind === "approval_resolved";
    case "governance":
      return GOVERNANCE_MOVED.has(kind);
    case "endpoint_view":
      return PROVIDER_MOVED.has(kind);
    case "inbox_view":
      return kind === "signal_enqueued" || kind === "signal_consumed";
    case "discard_view":
      return kind === "file_discarded" || kind === "discard_restored";
    case "registry_view":
      return kind === "asset_archived";
    case "cost_view": case "cost_of":
      return kind === "model_returned" || kind === "roadmap_claimed";
    // A prompt is frozen once for the life of a run and the object
    // behind a hash never changes, so neither answer can go stale.
    case "prefix": case "content":
      return false;
    // A shelf moves when somebody edits the building's rules, and a
    // pin appears when a run starts under them.
    case "skills":
      return kind === "building_configured" || kind === "run_started";
    // The working tree moves whenever a wave writes, and every wave
    // ends in a fence.
    case "git_status":
      return kind === "checkpoint_committed" || kind === "pr_merged";
    // Only the newest page can grow: an older page is bounded above by
    // a seq already written, and a commit's lineage walks backwards
    // from the run that made it, so a later successor never changes it.
    case "commits":
      return (kind === "checkpoint_committed" || kind === "pr_merged") && key.includes("\"before\":null");
    case "building_view": case "listing": case "document": case "archive_search":
      return BUILDING_MOVED.has(kind);
    default:
      return false;
  }
}

export interface Asking {
  // The answer to one question, as a signal: `undefined` until the
  // first answer lands. Asking inside a Solid owner counts as watching;
  // a watched answer is refreshed when it goes stale, an unwatched one
  // waits until somebody looks again.
  readonly ask: (query: Query) => Accessor<Answer | undefined>;
  readonly refresh: (query: Query) => void;
  readonly answered: (answer: Answer) => void;
  readonly invalidate: (record: EventRecord) => void;
  // Everything is stale after a reconnect: the page missed whatever
  // happened while the socket was down.
  readonly reconnected: () => void;
}

// The clock and the ear are handed in: a deadline a test cannot drive
// is a rule nobody has checked, and this module phrases nothing for a
// person - it names the `lang.json` phrase and the seam that knows the
// person's language says it.
export function createAsking(
  send: (query: Query) => boolean,
  now: () => number,
  noticed: (phrase: Key, error: Reported) => void,
): Asking {
  const held = new Map<string, Held>();
  const pending: Pending[] = [];
  const parsed = new Map<string, Query>();
  // One patience timer for the whole queue: the clock says which
  // questions are late, so a timer each would only be one more thing
  // to cancel.
  let patience: ReturnType<typeof setTimeout> | null = null;

  function watch(): void {
    const oldest = pending[0];
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
    for (let at = pending.length - 1; at >= 0; at -= 1) {
      const entry = pending[at];
      if (entry === undefined || entry.sentAt > deadline) continue;
      pending.splice(at, 1);
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
    if (!send(query)) {
      // Not live: it is asked again when the link comes back.
      slot.stale = true;
      return;
    }
    slot.inflight = true;
    slot.stale = false;
    pending.push({ key, query, sentAt: now() });
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
    if (found !== undefined) return [key, found];
    const [value, setValue] = createSignal<Answer | undefined>(undefined);
    const slot: Held = {
      value,
      set: (answer) => setValue(() => answer),
      reported: false,
      stale: true,
      inflight: false,
      watchers: 0,
      timer: null,
    };
    held.set(key, slot);
    parsed.set(key, query);
    return [key, slot];
  }

  function ask(query: Query): Accessor<Answer | undefined> {
    const [key, slot] = slotFor(query);
    if (getOwner() !== null) {
      slot.watchers += 1;
      onCleanup(() => {
        slot.watchers -= 1;
      });
    }
    if (slot.stale) dispatch(key, query, slot);
    return slot.value;
  }

  function refresh(query: Query): void {
    const [key, slot] = slotFor(query);
    slot.stale = true;
    dispatch(key, query, slot);
  }

  function settle(index: number, answer: Answer): void {
    const [done] = pending.splice(index, 1);
    if (done === undefined) return;
    const slot = held.get(done.key);
    if (slot === undefined) return;
    slot.inflight = false;
    slot.reported = false;
    slot.set(answer);
    if (slot.stale && slot.watchers > 0) schedule(done.key, done.query, slot);
  }

  function answered(answer: Answer): void {
    const key = keyOfAnswer(answer);
    const kinds = kindsOf(answer);
    const at = pending.findIndex((e) => (key === null ? kinds.includes(nameOf(e.query)) : e.key === key));
    if (at >= 0) {
      settle(at, answer);
      return;
    }
    // Nothing is waiting on it here, and a question whose patience ran
    // out has left the queue - but a late answer is still the answer,
    // so it lands on the slot that asked for it.
    const slot = key === null ? undefined : held.get(key);
    if (slot !== undefined) {
      slot.reported = false;
      slot.set(answer);
      return;
    }
    // Neither round could place it, so a question this page holds will
    // never be settled by its own answer, and the page would wait for
    // ever on an answer that has already been and gone.
    noticed("ask_unfiled", minted("E_WIRE_MISMATCH", "file an answer under the question it settles",
      Object.keys(answer)[0] ?? "answer"));
  }

  function invalidate(record: EventRecord): void {
    for (const [key, slot] of held) {
      const query = parsed.get(key);
      if (query === undefined || !staleBy(record, key, query)) continue;
      slot.stale = true;
      if (slot.watchers > 0) schedule(key, query, slot);
    }
  }

  function reconnected(): void {
    pending.length = 0;
    if (patience !== null) clearTimeout(patience);
    patience = null;
    for (const [key, slot] of held) {
      slot.inflight = false;
      slot.reported = false;
      slot.stale = true;
      const query = parsed.get(key);
      if (query !== undefined && slot.watchers > 0) dispatch(key, query, slot);
    }
  }

  return { ask, refresh, answered, invalidate, reconnected };
}
