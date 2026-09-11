// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this browser remembers about the person: the language they read
// and how hard the model should think when they say nothing. Neither is
// a fact about the city, so neither goes over the wire; both live in
// `localStorage` and are read once at start.

import { createSignal } from "solid-js";
import type { Accessor } from "solid-js";

import type { Lang } from "./lang";
import { langOf } from "./lang";
import type { Effort } from "../wire";

const LANG_KEY = "sprawling.lang";
const EFFORT_KEY = "sprawling.effort";
const WELCOMED_KEY = "sprawling.welcomed";
// One unsent message per place a person writes, kept across a reload
// or a page change; the key is the room or the run.
const DRAFT_PREFIX = "sprawling.draft.";

export const EFFORTS: readonly Effort[] = [
  "none",
  "low",
  "medium",
  "high",
  "xhigh",
  "max",
];

function readEffort(raw: string | null): Effort {
  return EFFORTS.find((effort) => effort === raw) ?? "medium";
}

function readLang(raw: string | null, fallback: string): Lang {
  return raw === "en" || raw === "zh" ? raw : langOf(fallback);
}

export interface Prefs {
  readonly lang: Accessor<Lang>;
  readonly setLang: (lang: Lang) => void;
  readonly effort: Accessor<Effort>;
  readonly setEffort: (effort: Effort) => void;
  // Whether this browser has walked through the welcome once. The city
  // decides whether setup is *needed*; this only decides whether a
  // person who skipped it is nagged again.
  readonly welcomed: Accessor<boolean>;
  readonly setWelcomed: (done: boolean) => void;
  // What was typed and not sent, by where it was typed. Not a signal:
  // the box that owns it reads it once when it mounts.
  readonly draft: (at: string) => string;
  readonly setDraft: (at: string, text: string) => void;
}

export function loadPrefs(store: Storage, browserLang: string): Prefs {
  const [lang, setLangSignal] = createSignal<Lang>(
    readLang(store.getItem(LANG_KEY), browserLang),
  );
  const [effort, setEffortSignal] = createSignal<Effort>(
    readEffort(store.getItem(EFFORT_KEY)),
  );
  const [welcomed, setWelcomedSignal] = createSignal<boolean>(
    store.getItem(WELCOMED_KEY) === "yes",
  );
  return {
    lang,
    setLang(next) {
      store.setItem(LANG_KEY, next);
      setLangSignal(next);
    },
    effort,
    setEffort(next) {
      store.setItem(EFFORT_KEY, next);
      setEffortSignal(next);
    },
    welcomed,
    setWelcomed(done) {
      store.setItem(WELCOMED_KEY, done ? "yes" : "no");
      setWelcomedSignal(done);
    },
    draft(at) {
      return store.getItem(DRAFT_PREFIX + at) ?? "";
    },
    setDraft(at, text) {
      if (text === "") {
        store.removeItem(DRAFT_PREFIX + at);
      } else {
        store.setItem(DRAFT_PREFIX + at, text);
      }
    },
  };
}
