// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What every view is handed: the connection, the person's preferences,
// the address bar, and the commands a page can send. One door - `setUi`
// opens it once as the shell mounts, `ui()` is the only way in - so a
// view reaches the browser through exactly what it was given.
//
// **Words are not context.** `core/lang.ts`'s `say` stays a pure
// function and the template writes `say($lang, key)`: the subscription
// to `lang` below is the whole reason a language change redraws a word.

import { derived, get, readable, writable } from "svelte/store";
import type { Readable } from "svelte/store";

import { QUERIES } from "./core/asking";
import { MODES } from "./core/commands";
import { createBelief } from "./core/belief";
import type { Lang } from "./core/lang";
import type { LinkState } from "./core/link";
import { loadPreferences } from "./core/prefs";
import type { PreferenceDoor } from "./core/prefs";
import { memory } from "./core/rows";
import type { AddressBar, View } from "./core/route";
import { go } from "./core/route";
import type { Connection } from "./core/socket";
import type { Answer, ApprovalItem, Command, Effort, Mode } from "./wire";

export interface Ui {
  readonly conn: Connection;
  readonly prefs: PreferenceDoor;
  // The language the page is drawn in. Every word subscribes to it
  // through `say($lang, key)`, which is the only thing that has to be
  // told when the person picks another language.
  readonly lang: Readable<Lang>;
  // How hard the model is asked to think in the session the next
  // dispatch opens, and `null` when nobody has said - which leaves the
  // field out of the frame, so the city's own `[model] effort` answers
  // and, failing that, the provider does.
  //
  // **It is held for this page and kept nowhere.** A level remembered
  // in this browser would ride on every dispatch made from it and
  // overrule the city's file without saying so; what the selector over
  // the composer states is the session it is about to open.
  readonly effort: Readable<Effort | null>;
  // The discipline the next dispatch runs under, held for this page the
  // way effort is; it starts at the first a control offers.
  readonly mode: Readable<Mode>;
  // The questions this city is holding for the person, as one reading
  // three views share: the dot on the rail, the rail's badge, and the
  // tab's title. Before this each of them folded the same answer by
  // hand, and a fourth reader would have folded it a fourth time.
  // `undefined` until the city first answers: an empty list is a
  // snapshot, and a reader that took the placeholder for one (the
  // notifier's first snapshot) would raise every item that follows.
  readonly approvals: Readable<readonly ApprovalItem[] | undefined>;
  readonly bar: AddressBar;
  readonly origin: string;
  // The pairing code this page was opened with, as the city's HTTP
  // doors ask for it (`core/socket.ts` reads it off the address bar
  // and spells the header). `null` on a city that configured none,
  // which is a city on loopback.
  readonly pairing: string | null;
  // Milliseconds now, read where a view needs a relative time.
  readonly now: () => number;
  readonly chooseEffort: (level: Effort | null) => void;
  readonly chooseMode: (mode: Mode) => void;
  readonly go: (view: View) => void;
  // Sends a command, and says so when it could not be sent.
  readonly send: (command: Command) => boolean;
  // Whether this city has a model chosen to turn speech into text.
  // The composer draws no microphone without one: a control whose only
  // possible answer is a refusal is a control nobody should meet.
  readonly hearing: () => boolean;
}

// What `main.ts` reads once off the page and hands down: the link, the
// door onto the person's preferences, and the facts an address bar and
// an origin carry. Every reading a view takes beyond these is folded
// from them here, so a fold has one home.
export interface Opening {
  readonly conn: Connection;
  readonly prefs: PreferenceDoor;
  readonly bar: AddressBar;
  readonly origin: string;
  readonly pairing: string | null;
  readonly now: () => number;
}

// The folds, built in one place: the shell's door and the orphaned view
// below must arrive at the same `Ui`, or an error path becomes a second
// statement of what a page reads.
function readied(value: Opening): Ui {
  const effort = writable<Effort | null>(null);
  const mode = writable<Mode>(MODES[0] ?? "plan_goal");
  const hearing = value.conn.asking.ask(QUERIES.endpoints);
  return {
    ...value,
    lang: derived(value.prefs.held, (held) => held.lang),
    effort,
    mode,
    approvals: derived(
      value.conn.asking.ask(QUERIES.approvals),
      (answer) => (answer !== undefined && "approvals" in answer ? answer.approvals.items : undefined),
    ),
    chooseEffort: effort.set,
    chooseMode: mode.set,
    go: (view) => {
      go(value.bar, view);
    },
    send: value.conn.command,
    hearing: () => {
      const answer: Answer | undefined = get(hearing);
      return (
        answer !== undefined && "endpoints" in answer &&
        answer.endpoints.chosen.some((each) => each.tag === "transcribe")
      );
    },
  };
}

let opened: Ui | null = null;

// The one call that opens the door, made as the shell mounts.
export function setUi(value: Opening): void {
  opened = readied(value);
}

// What a view mounted outside the shell is handed. Reaching this is a
// programming error rather than a state, so the page draws with an
// idle socket, a store that lasts as long as the call, and the
// postures the client ships with - none of which this file spells:
// `loadPreferences` answers them from an empty store, and the belief is
// the constructor's own empty value, so an error path cannot become a
// second statement of a default.
export function ui(): Ui {
  if (opened !== null) {
    return opened;
  }
  return readied({
    conn: {
      state: readable<LinkState>({ kind: "idle" }),
      belief: createBelief(() => 0).belief,
      live: readable({}),
      asking: {
        ask: () => readable<Answer | undefined>(undefined),
        refresh: () => undefined,
        answered: () => undefined,
        invalidate: () => undefined,
        reconnected: () => undefined,
        resumed: () => undefined,
      },
      unsent: readable(0),
      command: () => false,
      retry: () => undefined,
      dismissRefusal: () => undefined,
      markNoticesSeen: () => undefined,
      monitor: { samples: readable([]), watch: () => () => undefined, watchSummary: () => () => undefined },
    },
    prefs: loadPreferences(memory(), ""),
    bar: { hash: "" },
    origin: "",
    pairing: null,
    now: () => 0,
  });
}
