// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one recovery offers a person: the label on its control, when
// that control may be pressed, and what pressing it does. `core/
// recovering.ts` says which actions a refusal offers; this file is the
// view's single answer to what they mean, read by both notice seats -
// the drawer and the toast (client-SPEC 4-35). Two seats that each
// wired a deed would give one refusal two answers.
//
// **A command recovery is labelled in the person's own words and does
// the command's deed directly.** The phrase travels as a `lang.json`
// key (`act_*`), and the deed is the command frame behind the spelling
// (`/new` is `open_session`, `/stop` is `cancel`), so a
// notice reaches the city without a line passing through a text box.
// `/fork` is the one exception: a branch is chosen at a point, and the
// picker that chooses it lives on the talk screen (ux A6), so its deed
// lands on that screen, in the room the refusal names. A spelling this
// file does not know runs nothing rather than a guess at what it meant.

import { Option, Schema } from "effect";
import { get } from "svelte/store";

import { cancel, openSession } from "../core/commands";
import type { Belief } from "../core/belief";
import type { Key, Lang } from "../core/lang";
import { say } from "../core/lang";
import type { Recovery } from "../core/recovering";
import { readRunId } from "../core/run_id";
import type { Ui } from "../ui";
import { Address } from "../wire";
import type { RunId } from "../wire";

// What a refusal's subject names, when this build can name it. The same
// reading `core/belief` stamps onto a notice.
export type About = Address | RunId | null;

// The phrase each command recovery is offered under, keyed by the
// spelling `core/recovering.ts` hands out. A spelling this table does
// not name shows as itself: a visible `/oddity` is a defect somebody
// reports, and a silently chosen default word is one nobody does.
const VERBS: Readonly<Record<string, Key>> = {
  "/new": "act_open_session",
  "/fork": "act_fork",
  "/stop": "talk_stop",
};

// Why a deed cannot run: the refusal named nothing it can act on.
const NO_TARGET: Key = "act_no_target";

// The two things a subject can name that a deed can act on: the run
// itself, and the room a new session opens in. A run whose room the
// page has never heard of still stops, but has nowhere to open.
interface Target {
  readonly run: RunId | null;
  readonly room: Address | null;
}

function targetOf(about: About, runs: Belief["runs"]): Target {
  if (about === null) {
    return { run: null, room: null };
  }
  // The run grammar is asked first because it is far narrower: a
  // one-segment room name would otherwise swallow every run
  // (`core/belief/shape` reads a subject the same way).
  const run = Option.getOrNull(readRunId(about));
  if (run !== null) {
    return { run, room: runs[run]?.addr ?? null };
  }
  return { run: null, room: Option.getOrNull(Schema.decodeOption(Address)(about)) };
}

function targetFor(u: Ui, about: About): Target {
  return targetOf(about, get(u.conn.belief).runs);
}

// The label a control for one recovery carries, in the person's own
// language. A reconnect is labelled by the key that travels with it;
// a command is labelled by its spelling's own phrase where the table
// above knows one.
export function recoveryLabel(recovery: Recovery, lang: Lang): string {
  if (recovery.kind === "reconnect") {
    return say(lang, recovery.verb);
  }
  const key = VERBS[recovery.spelled];
  return key === undefined ? recovery.spelled : say(lang, key);
}

// Why a recovery cannot be pressed right now, as a `lang.json` key - or
// `undefined` when it can. The subject is what every deed acts on, so a
// refusal whose subject is a sentence offers its deeds greyed rather
// than guessing at a room (SPEC 7-2: `aria-disabled` keeps the control
// in the Tab order with its reason readable).
export function recoveryWhy(u: Ui, recovery: Recovery, about: About): Key | undefined {
  if (recovery.kind === "reconnect") {
    return undefined;
  }
  const target = targetFor(u, about);
  if (recovery.spelled === "/stop") {
    return target.run === null ? NO_TARGET : undefined;
  }
  return target.room === null ? NO_TARGET : undefined;
}

// What one recovery does. `/new` opens the session itself - `open_session`
// is the command behind `/new`, sent without a line through a box - and
// `/stop` cancels the run the refusal names and nothing wider: halting a
// room is `/halt`, a separate verb (client-SPEC 4-39).
export function recover(u: Ui, recovery: Recovery, about: About): void {
  if (recovery.kind === "reconnect") {
    u.conn.retry();
    return;
  }
  const target = targetFor(u, about);
  switch (recovery.spelled) {
    case "/new":
      if (target.room !== null) {
        u.send(openSession(target.room, "nothing", null));
      }
      return;
    case "/fork":
      if (target.room !== null) {
        u.go({ kind: "talk", address: target.room });
      }
      return;
    case "/stop":
      if (target.run !== null) {
        u.send(cancel(target.run));
      }
      return;
    default:
      // `core/recovering.ts` names four spellings and no others; a
      // fifth is a defect in that table, and this arm keeps it a no-op
      // rather than a guess at what it meant.
      return;
  }
}
