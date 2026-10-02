// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one recovery offers a person: the label on its control, when
// that control may be pressed, and what pressing it does. `core/
// recovering.ts` says which actions a refusal offers; this file is the
// view's single answer to what they mean, read by both notice seats -
// the drawer and the toast (client/Spec.lean §4-35). Two seats that each
// wired a deed would give one refusal two answers.
//
// **A command recovery is labelled in the person's own words and does
// the command's deed directly.** The phrase travels as a `lang.json`
// key (`act_*`), and the deed is the command frame behind the spelling
// (`/new` is `open_session`, `/stop` is `cancel`), so a
// notice reaches the city without a line passing through a text box.
// `/fork` is the one exception: a branch is chosen at a point, and the
// picker that chooses it lives on the talk screen (ux A6), so its deed
// lands on that screen. A spelling this file does not know runs nothing
// rather than a guess at what it meant.
//
// **A deed acts on the composer's room, never on the subject's words.**
// The room is the one the address bar stands in - a conversation's
// address, or the room of the run a run page follows - because that is
// where the refused request was sent from. The address grammar accepts
// a sentence with spaces and backquotes, so a subject read as a room
// once opened a building named after an error message (client/Spec.lean §4-35).
//
// **A form recovery writes and moves; it never sends** (client/Spec.lean
// §4-35a). It fills the draft door of the room it names and moves there,
// and the dispatch that follows is the one the person presses.

import { Option } from "effect";
import { get } from "svelte/store";

import { cancel, openSession } from "../core/commands";
import type { Belief } from "../core/belief";
import type { Key, Lang } from "../core/lang";
import { say } from "../core/lang";
import type { Recovery } from "../core/recovering";
import { formOf } from "../core/recovering";
import { current } from "../core/route";
import type { View } from "../core/route";
import { readRunId } from "../core/run_id";
import type { Ui } from "../ui";
import type { Address, AxError, RunId } from "../wire";

// What a refusal's subject names, when this build can name it. The same
// reading `core/belief` stamps onto a notice.
export type About = Address | RunId | null;

// One refusal as a notice holds it: the city's own error, and what its
// subject names.
export interface Refused {
  readonly error: AxError;
  readonly about: About;
}

// The phrase each command recovery is offered under, keyed by the
// spelling `core/recovering.ts` hands out. A spelling this table does
// not name shows as itself: a visible `/oddity` is a defect somebody
// reports, and a silently chosen default word is one nobody does.
const VERBS: Readonly<Record<string, Key>> = {
  "/new": "act_open_session",
  "/fork": "act_fork",
};

// Why a deed cannot run: the refusal named nothing it can act on.
const NO_TARGET: Key = "act_no_target";

// What a deed can act on: the run the subject names, and the room the
// composer stands in. Only the run is read off the subject, because the
// run grammar is too narrow to hold a sentence; a run page's run whose
// room this page has never heard of still stops, but has nowhere to open.
interface Target {
  readonly run: RunId | null;
  readonly room: Address | null;
}

function targetFor(u: Ui, about: About): Target {
  const runs = get(u.conn.belief).runs;
  return {
    run: about === null ? null : Option.getOrNull(readRunId(about)),
    room: Option.match(current(u.bar), {
      onNone: () => null,
      onSome: (view) => composerRoom(view, runs),
    }),
  };
}

// The room a page's composer sends to, and `null` on a page that has
// no composer.
function composerRoom(view: View, runs: Belief["runs"]): Address | null {
  if (view.kind === "talk") {
    return view.address;
  }
  if (view.kind === "run") {
    return runs[view.run]?.addr ?? null;
  }
  return null;
}

// The label a control for one recovery carries, in the person's own
// language. A reconnect is labelled by the key that travels with it;
// a command is labelled by its spelling's own phrase where the table
// above knows one.
export function recoveryLabel(recovery: Recovery, lang: Lang): string {
  switch (recovery.kind) {
    case "reconnect":
    case "settings":
    case "reload":
      return say(lang, recovery.verb);
    case "form":
      return say(lang, recovery.label);
    case "command":
      return commandLabel(recovery.spelled, lang);
  }
}

function commandLabel(spelled: string, lang: Lang): string {
  const key = VERBS[spelled];
  return key === undefined ? spelled : say(lang, key);
}

// Why a recovery cannot be pressed right now, as a `lang.json` key - or
// `undefined` when it can. A page without a composer has no room to act
// on, so it offers its deeds greyed rather than guessing at one (SPEC 7-2: `aria-disabled` keeps the control
// in the Tab order with its reason readable).
export function recoveryWhy(u: Ui, recovery: Recovery, refused: Refused): Key | undefined {
  if (recovery.kind === "reconnect" || recovery.kind === "settings" || recovery.kind === "reload") {
    return undefined;
  }
  if (recovery.kind === "form") {
    return Option.isNone(formOf(recovery.room, refused.error.subject, "")) ? NO_TARGET : undefined;
  }
  const target = targetFor(u, refused.about);
  if (recovery.spelled === "/stop") {
    return target.run === null ? NO_TARGET : undefined;
  }
  return target.room === null ? NO_TARGET : undefined;
}

// What one recovery does. `/new` opens the session itself - `open_session`
// is the command behind `/new`, sent without a line through a box - and
// `/stop` cancels the run the refusal names and nothing wider: halting a
// room is `/halt`, a separate verb (client/Spec.lean §4-41).
export function recover(u: Ui, recovery: Recovery, refused: Refused): void {
  if (recovery.kind === "reconnect") {
    u.conn.retry();
    return;
  }
  // The settings recovery is "choose a model", which is the accounts
  // group's model choice.
  if (recovery.kind === "settings") {
    u.go({ kind: "setup", group: "accounts" });
    return;
  }
  if (recovery.kind === "reload") {
    location.reload();
    return;
  }
  if (recovery.kind === "form") {
    const words = say(get(u.lang), recovery.words);
    Option.map(formOf(recovery.room, refused.error.subject, words), (form) => {
      u.prefs.setDraft(form.room, form.draft);
      u.go({ kind: "talk", address: form.room });
    });
    return;
  }
  const target = targetFor(u, refused.about);
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
