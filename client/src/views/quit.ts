// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the `/quit` question offers (`quit.svelte`): which sentence
// explains it, and which answers besides cancel it gives.

import type { Key } from "../core/lang";
import type { CloseMode } from "../wire";
import type { Tone } from "./parts/button";

export interface QuitAnswer {
  readonly mode: CloseMode;
  readonly label: Key;
  readonly tone: Tone;
}

export interface QuitLook {
  readonly about: Key;
  readonly runs: number;
  readonly answers: readonly QuitAnswer[];
}

export interface QuitAsked {
  // Runs still going in the city.
  readonly runs: number;
  // The page's origin: the remote door is https, this machine's port is not.
  readonly here: string;
}

export function quitOf(asked: QuitAsked): QuitLook {
  if (asked.here.startsWith("https:")) return { about: "quit_local_only", runs: 0, answers: [] };
  if (asked.runs === 0) {
    return { about: "quit_about", runs: 0, answers: [{ mode: "drain", label: "quit_close", tone: "destructive" }] };
  }
  return {
    about: "quit_about",
    runs: asked.runs,
    answers: [
      { mode: "drain", label: "quit_wait", tone: "primary" },
      { mode: "interrupt", label: "quit_now", tone: "destructive" },
    ],
  };
}
