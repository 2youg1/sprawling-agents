// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a place that opens sessions says about one while the city has
// not opened it (client D100): that it is opening, or why the city
// refused it, with `/stop` beside a refusal that a working run caused.
// The mailbox row and the earlier session in main say it in the same
// words, read here; a branch and a fresh start (`/new`, `/compact`) are
// named apart, because the person asked for different things.

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { STOP } from "../../core/slash";
import { noticeTitle } from "../parts/notice_title";
import type { Address } from "../../wire";
import type { BranchHere, Opening } from "./branching";

// The words for each kind of opening: while it opens, refused because a
// run is working, and refused for any other reason.
const WORDS: Record<Opening["kind"], { readonly opening: Key; readonly busy: Key; readonly refused: Key }> = {
  branch: { opening: "branch_opening", busy: "branch_busy", refused: "branch_refused" },
  fresh: { opening: "session_opening", busy: "session_busy", refused: "session_refused" },
};

export interface BranchNote {
  readonly said: string;
  // `/stop` for the run that keeps the room busy; absent when the
  // refusal was about something else, or the run has stopped since.
  readonly stop: StopKey | null;
}

export interface StopKey {
  readonly label: string;
  readonly name: string;
  readonly onPress: () => void;
}

// `stop` cancels the run working in the room now, or is `null` when
// none is.
export function noteOf(here: BranchHere, room: Address, lang: Lang, stop: (() => void) | null): BranchNote | null {
  switch (here.kind) {
    case "none":
    case "elsewhere":
      return null;
    case "opening":
      return { said: say(lang, WORDS[here.of].opening), stop: null };
    case "refused": {
      const error = here.error;
      const words = WORDS[here.of];
      if (error.code !== "E_BUSY") {
        return { said: fill(say(lang, words.refused), { why: noticeTitle(lang, error.code, error.subject) }), stop: null };
      }
      return {
        said: say(lang, words.busy),
        stop: stop === null ? null : { label: STOP, name: fill(say(lang, "branch_stop"), { room }), onPress: stop },
      };
    }
  }
}

// Why a fork key cannot be pressed now: its own branch is opening, or
// another one is; `undefined` when it can.
export function waitWhy(here: BranchHere): Key | undefined {
  switch (here.kind) {
    case "opening":
      return "branch_opening";
    case "elsewhere":
      return "branch_one_at_a_time";
    case "none":
    case "refused":
      return undefined;
  }
}
