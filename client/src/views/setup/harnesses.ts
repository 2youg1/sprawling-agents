// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one harness card says about its state (`crates/wire/spec/Answer/
// Harnesses.lean`): the launcher is missing, the launcher is here but the
// harness is not installed or not signed in, or it is ready. A launcher
// on the search path is not a harness: every npm-launched harness starts
// from the same `npx`, so the middle state names the directories the
// city looked in, and a User can see which one is empty.

import { say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import type { Status } from "../parts/glyph";
import type { HarnessLine, HarnessState } from "../../wire";

export interface HarnessReading {
  readonly key: Key;
  readonly status: Status;
  // The program that is missing, or where the harness was found.
  readonly said: string | null;
  // The directories looked in when the harness was not found there.
  readonly looked: readonly string[];
}

export function harnessOf(state: HarnessState): HarnessReading {
  if ("launcher_missing" in state) {
    return { key: "harness_launcher_missing", status: "idle", said: state.launcher_missing.program, looked: [] };
  }
  if ("not_set_up" in state) {
    return { key: "harness_not_set_up", status: "waiting", said: null, looked: state.not_set_up.looked };
  }
  return { key: "harness_ready", status: "done", said: state.ready.at, looked: [] };
}

// The names the vendors give their harnesses.
// wording-ok: product names, which no language translates
const NAMES: Readonly<Record<string, string>> = {
  claude_code: "Claude Code",
  codex: "Codex",
  grok_build: "Grok Build",
  kimi_code: "Kimi Code",
  pi: "Pi",
};

// One card as a look draws it: every word translated, the command and
// the sign-in page as the city stated them. The person signs in inside
// the harness, and the city never asks for that credential.
export interface HarnessCardLook {
  readonly key: string;
  readonly name: string;
  readonly state: string;
  readonly status: Status;
  readonly said: string | null;
  // The heading over the directories looked in, present only with them.
  readonly lookedIn: string | undefined;
  readonly looked: readonly string[];
  readonly launch: string;
  readonly docs: { readonly href: string; readonly label: string };
}

export function cardsOf(lines: readonly HarnessLine[], lang: Lang): readonly HarnessCardLook[] {
  return lines.map((line) => {
    const reading = harnessOf(line.state);
    return {
      key: line.name,
      name: NAMES[line.name] ?? line.name,
      state: say(lang, reading.key),
      status: reading.status,
      said: reading.said,
      lookedIn: reading.looked.length > 0 ? say(lang, "harness_looked") : undefined,
      looked: reading.looked,
      launch: line.launch.join(" "),
      docs: { href: line.docs, label: say(lang, "harness_sign_in") },
    };
  });
}
