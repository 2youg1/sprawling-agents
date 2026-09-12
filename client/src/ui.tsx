// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What every view is handed: the connection, the person's preferences,
// the address bar, and the one function that turns a key into words.
// Passed as context rather than imported, so a view reaches the browser
// through exactly what it was given.

import { createContext, useContext } from "solid-js";

import type { Key, Lang } from "./core/lang";
import { fill, say } from "./core/lang";
import type { Prefs } from "./core/prefs";
import type { AddressBar, View } from "./core/route";
import { go } from "./core/route";
import type { Connection } from "./core/socket";
import type { Command } from "./wire";

export interface Ui {
  readonly conn: Connection;
  readonly prefs: Prefs;
  readonly bar: AddressBar;
  readonly origin: string;
  // Milliseconds now, read where a view needs a relative time.
  readonly now: () => number;
}

const UiContext = createContext<Ui>();

export const UiProvider = UiContext.Provider;

export function useUi(): Ui {
  const ui = useContext(UiContext);
  if (ui === undefined) {
    // Only reachable by a view mounted outside the provider, which is a
    // programming error rather than a state; the page has nothing to
    // draw without it.
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
      prefs: {
        lang: () => "en",
        setLang: () => undefined,
        effort: () => "medium",
        setEffort: () => undefined,
        welcomed: () => false,
        setWelcomed: () => undefined,
        draft: () => "",
        setDraft: () => undefined,
      },
      bar: { hash: "" },
      origin: "",
      now: () => 0,
    };
  }
  return ui;
}

// The word for one key in the person's language, with its slots filled.
export function useSay(): (key: Key, slots?: Readonly<Record<string, string>>) => string {
  const ui = useUi();
  return (key, slots) => {
    const phrase = say(ui.prefs.lang(), key);
    return slots === undefined ? phrase : fill(phrase, slots);
  };
}

export function useLang(): () => Lang {
  return useUi().prefs.lang;
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

// Whether this city has a model chosen to turn speech into text.
//
// The composer draws no microphone without one: a control whose only
// possible answer is a refusal is a control nobody should meet. It is a
// hook rather than a prop threaded through the pages because two
// composers ask the same question of the same answer.
export function useHearing(): () => boolean {
  const ui = useUi();
  const endpoints = ui.conn.asking.ask("endpoint_view");
  return () => {
    const held = endpoints();
    if (held === undefined || !("endpoints" in held)) return false;
    return held.endpoints.chosen.some((each) => each.tag === "transcribe");
  };
}
