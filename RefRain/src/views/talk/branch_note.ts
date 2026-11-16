// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { STOP } from "../../core/slash";
import { noticeTitle } from "../parts/notice_title";
import type { Address } from "../../wire";
import type { BranchHere, Opening } from "./branching";

const WORDS: Record<
  Opening["kind"],
  { readonly opening: Key; readonly busy: Key; readonly refused: Key; readonly refusedThen: Key }
> = {
  branch: {
    opening: "branch_opening",
    busy: "branch_busy",
    refused: "branch_refused",
    refusedThen: "branch_refused_then",
  },
  fresh: {
    opening: "session_opening",
    busy: "session_busy",
    refused: "session_refused",
    refusedThen: "session_refused_then",
  },
};

export interface BranchNote {
  readonly said: string;
  readonly stop: StopKey | null;
}

export interface StopKey {
  readonly label: string;
  readonly name: string;
  readonly onPress: () => void;
}

export function noteOf(
  here: BranchHere,
  room: Address,
  lang: Lang,
  stop: (() => void) | null,
): BranchNote | null {
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
        const why = noticeTitle(lang, error.code, error.subject);
        const said =
          error.recovery === ""
            ? fill(say(lang, words.refused), { why })
            : fill(say(lang, words.refusedThen), { why, recovery: error.recovery });
        return { said, stop: null };
      }
      return {
        said: say(lang, words.busy),
        stop:
          stop === null
            ? null
            : { label: STOP, name: fill(say(lang, "branch_stop"), { room }), onPress: stop },
      };
    }
  }
}

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
