// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The shapes the belief holds, in a file of their own.
//
// **What a store holds is a declaration, not behaviour**: it depends on
// no store, no setter and no socket, so it leaves `belief.ts` the room a
// new fact about a session needs. The two window constants live here
// with the fields they bound, which is the one place the question "how
// much of this does the page keep" is answered.
//
// The one rule that decides when two refusals are the same refusal lives
// beside the `Notice` type for the same reason: what makes two notices
// one notice is a property of the shape, and `belief.ts` folds it in
// without restating the rule.

import { Option, Schema } from "effect";

import { Address } from "../../wire";
import type {
  AxError,
  HaltScope,
  LogLine,
  RunId,
  Seq,
  TimeMs,
} from "../../wire";
import type { Doing } from "../doing";
import type { Probed } from "../probed";
import { readRunId } from "../run_id";

export interface RunBelief {
  readonly run: RunId;
  readonly addr: Address | null;
  readonly started: TimeMs | null;
  // What the person asked for and what finishing looks like, as the
  // run's opening said them: heard from the stream, or read off the
  // city's answer after a reload.
  readonly task: string | null;
  readonly goal: string | null;
  readonly lastSeq: Seq;
  readonly doing: Doing;
  // The model the run's last call went to. A session keeps the model it
  // was opened with, so this, and not the city's next pick, is the model
  // answering; null until the run's first call is heard.
  readonly model: string | null;
  // The branch of the last pull request the run opened: a pull request
  // in the city is named by its branch and has no number.
  readonly pr: string | null;
  // What the run waits for the person to allow, present only while its
  // last record is the request, as `RunSummary.ask` is.
  readonly ask: string | null;
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

// What the belief holds. Read through the store `belief.ts` hands out;
// nothing outside that file writes one.
export interface Belief {
  // The live run table, written in place by the store: a belief read
  // earlier sees later runs through it, so a reader that needs a run as
  // it stood keeps the `RunBelief`, which is never mutated.
  runs: Record<string, RunBelief>;
  // The runs that have not frozen, oldest start first: the one answer
  // to which runs are working (`live.ts`).
  live: readonly RunBelief[];
  // The ids of the runs each room has held, oldest start first
  // (`rooms.ts`).
  rooms: ReadonlyMap<string, readonly RunId[]>;
  // How many runs froze cancelled (`cancelled.ts`).
  cancelled: number;
  halted: HaltScope[];
  // The ledger position the list of shut scopes is current to. Two
  // writers touch the list - an answer states the whole of it and a
  // record changes one scope - so without a position a gap folded back
  // in after the answer would undo it.
  haltedAt: Seq;
  // The last refusal a command came back with, for the page to show
  // once and the person to dismiss.
  refusal: AxError | null;
  // Every refusal this session has seen, oldest first, bounded. The
  // dismissed one leaves the corner and stays here: a refusal a person
  // waved away is still the answer to what they asked.
  notices: Notice[];
  city: string | null;
  // Where the newest session in each room began (core/session.ts reads it).
  sessions: Record<string, Seq>;
  // Every document a card was offered on while this page listened, and
  // where the newest offer sits (`proposed.ts`). It says somebody
  // proposed a change there, not how many cards are still open.
  proposed: ReadonlyMap<Address, Seq>;
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
export const LOG_WINDOW = 500;

// One refusal, kept after the corner has let go of it. A refusal that
// arrives again as itself is the same notice, counted: two dispatches
// refused by one frozen shape used to draw the same four fields twice,
// and a person could not tell a city that failed once from one that
// failed twenty times (client-SPEC 4-35).
export interface Notice {
  readonly error: AxError;
  // Whether the bell has been read since the last arrival. A repeat
  // arrival is news again, so the merge clears it.
  seen: boolean;
  // When the first of them was seen. A repeat keeps it: the drawer
  // groups by day, and one refusal belongs to one day.
  readonly at: TimeMs;
  // What makes two refusals one refusal: the code and the subject
  // together, because one code covers several subjects and the subject
  // is what a person acts on.
  readonly key: string;
  // How many times this refusal has arrived. The newest sentence rides
  // with the count: `error` is the city's latest word.
  count: number;
  // What the subject names, when this build can name it: a run or a
  // room. `null` when the subject is a sentence rather than a name.
  readonly about: Address | RunId | null;
}

// How many refusals the bell keeps. Short on purpose: this is a list a
// person reads, not a record they audit - the ledger is where a city's
// history lives.
export const NOTICE_WINDOW = 50;

// The identity of one refusal. NUL between the two halves because it is
// the one character neither a code nor a subject can carry.
function noticeKey(error: AxError): string {
  return `${error.code}\u0000${error.subject}`;
}

// The address grammar is the server's, carried in the schema `cargo
// xtask wire-ts` generates; a subject is read through it and nowhere
// else. A run id is asked first because its grammar is far narrower: a
// room with a one-segment name would otherwise swallow every run.
const readAddress = Schema.decodeOption(Address);

function about(subject: string): Address | RunId | null {
  const run = readRunId(subject);
  if (Option.isSome(run)) return run.value;
  return Option.getOrElse(readAddress(subject), () => null);
}

// One refusal in, answered with the list it belongs to. The same
// refusal keeps its first-seen time and its place in the list - the
// list is ordered by when each refusal first arrived - while the count,
// the sentence and the read mark move to now. A new refusal closes the
// window by dropping the oldest.
export function merged(notices: readonly Notice[], error: AxError, at: TimeMs): Notice[] {
  const key = noticeKey(error);
  const held = notices.find((each) => each.key === key);
  if (held === undefined) {
    const out = [...notices, { error, seen: false, at, key, count: 1, about: about(error.subject) }];
    return out.length > NOTICE_WINDOW ? out.slice(out.length - NOTICE_WINDOW) : out;
  }
  return notices.map((each) =>
    each.key === key ? { ...each, error, seen: false, count: each.count + 1 } : each,
  );
}

// A run this page heard of and has no record of: a gap page starting
// after the beginning, or text that outran the run it belongs to. The
// position is before the first record (`Seq::FIRST`) and the phase is
// not yet a fact the stream has stated.
