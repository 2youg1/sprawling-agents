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

import type { Key } from "../../core/lang";
import type { Status } from "../parts/glyph";
import type { HarnessState } from "../../wire";

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
