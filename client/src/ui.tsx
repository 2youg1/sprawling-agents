// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What every view is handed: the connection, the person's preferences,
// the address bar, and the one function that turns a key into words.
// Passed as context rather than imported, so a view reaches the browser
// through exactly what it was given.

import { createContext, createMemo, createSignal, useContext } from "solid-js";
import type { Accessor } from "solid-js";

import { QUERIES } from "./core/asking";
import { putPreferences } from "./core/commands";
import type { Key, Lang } from "./core/lang";
import { fill, say } from "./core/lang";
import { loadPreferences } from "./core/prefs";
import type { PreferenceDoor } from "./core/prefs";
import { memory } from "./core/rows";
import type { AddressBar, View } from "./core/route";
import { go } from "./core/route";
import type { Connection } from "./core/socket";
import type { ApprovalItem, Command, Effort } from "./wire";

export interface Ui {
  readonly conn: Connection;
  readonly prefs: PreferenceDoor;
  // How hard the model is asked to think in the session the next
  // dispatch opens, and `null` when nobody has said - which leaves the
  // field out of the frame, so the city's own `[model] effort` answers
  // and, failing that, the provider does.
  //
  // **It is held for this page and kept nowhere.** A level remembered
  // in this browser would ride on every dispatch made from it and
  // overrule the city's file without saying so; what the selector over
  // the composer states is the session it is about to open.
  readonly effort: Accessor<Effort | null>;
  readonly chooseEffort: (level: Effort | null) => void;
  readonly bar: AddressBar;
  readonly origin: string;
  // The pairing code this page was opened with, as the city's HTTP
  // doors ask for it (`core/socket.ts` reads it off the address bar
  // and spells the header). `null` on a city that configured none,
  // which is a city on loopback.
  readonly pairing: string | null;
  // Milliseconds now, read where a view needs a relative time.
  readonly now: () => number;
}

const UiContext = createContext<Ui>();

export const UiProvider = UiContext.Provider;

// What a view mounted outside the provider is handed. Reaching this is
// a programming error rather than a state, so the page draws with an
// idle socket, a store that lasts as long as the call, and the
// postures the client ships with - none of which this file spells:
// `loadPreferences` answers them from an empty store, so an error path
// cannot become a second statement of a default.
function orphaned(): Ui {
  const [effort, chooseEffort] = createSignal<Effort | null>(null);
  return {
    conn: {
      state: () => ({ kind: "idle" }),
      belief: { runs: {}, halted: [], refusal: null, notices: [], city: null, probed: null, logs: [] },
      asking: {
        ask: () => () => undefined,
        refresh: () => undefined,
        answered: () => undefined,
        invalidate: () => undefined,
        reconnected: () => undefined,
      },
      command: () => false,
      retry: () => undefined,
      dismissRefusal: () => undefined,
      markNoticesSeen: () => undefined,
    },
    prefs: loadPreferences(memory(), ""),
    effort,
    chooseEffort,
    bar: { hash: "" },
    origin: "",
    pairing: null,
    now: () => 0,
  };
}

export function useUi(): Ui {
  return useContext(UiContext) ?? orphaned();
}

// The word for one key in the person's language, with its slots filled.
export function useSay(): (key: Key, slots?: Readonly<Record<string, string>>) => string {
  const ui = useUi();
  return (key, slots) => {
    const phrase = say(ui.prefs.held().lang, key);
    return slots === undefined ? phrase : fill(phrase, slots);
  };
}

export function useLang(): () => Lang {
  const ui = useUi();
  return () => ui.prefs.held().lang;
}

export function useGo(): (view: View) => void {
  const ui = useUi();
  return (view) => {
    go(ui.bar, view);
  };
}

// Sends a command, and says so when it could not be sent.
export function useCommand(): (command: Command) => boolean {
  const ui = useUi();
  return (command) => ui.conn.command(command);
}

// Picking a language, everywhere it is kept. Two places hold it and
// this is the only caller that knows both: the browser's cache draws
// the next first paint without waiting for the socket, and the
// person's own `~/.sprawling/config.toml` is what a second browser
// reaching the same city reads. A hook rather than a line repeated at
// each picker, because the palette and the settings page offer the
// same choice and one of them would fall behind.
export function useLanguage(): (lang: Lang) => void {
  const ui = useUi();
  return (lang) => {
    ui.prefs.setLang(lang);
    ui.conn.command(putPreferences({ lang }));
  };
}

// Whether this city has a model chosen to turn speech into text.
//
// The composer draws no microphone without one: a control whose only
// possible answer is a refusal is a control nobody should meet. It is a
// hook rather than a prop threaded through the pages because two
// composers ask the same question of the same answer.
export function useHearing(): () => boolean {
  const ui = useUi();
  const endpoints = ui.conn.asking.ask(QUERIES.endpoints);
  return () => {
    const held = endpoints();
    if (held === undefined || !("endpoints" in held)) return false;
    return held.endpoints.chosen.some((each) => each.tag === "transcribe");
  };
}

// The questions this city is holding for the person, as one reading
// three views share: the dot on the rail, the rail's badge, and the
// tab's title. Before this each of them folded the same answer by
// hand, and a fourth reader would have folded it a fourth time.
//
// It is a hook rather than a prop because the three call sites are in
// three different places on the page - the same reason `useHearing`
// above it is one.
export function useApprovals(): Accessor<readonly ApprovalItem[]> {
  const ui = useUi();
  const answer = ui.conn.asking.ask(QUERIES.approvals);
  return createMemo<readonly ApprovalItem[]>(() => {
    const held = answer();
    return held !== undefined && "approvals" in held ? held.approvals.items : [];
  });
}
