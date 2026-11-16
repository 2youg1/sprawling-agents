// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The uses of one skill or one server's tool, as the usage panels on
// the skill page and the MCP page both draw them: the count and the
// last use on one line, the counts per UTC day, and every use - run,
// room, moment, the part read, how the call ended - folded under a
// disclosure, because a skill read a thousand times would otherwise
// push every other row off the page. The city folds the table
// (`Query::SkillUsage`, `Query::McpUsage`); this part only draws it.

import type { Key } from "../../core/lang";
import type { DayCount, UseOutcome } from "../../wire";

export interface UseRow {
  readonly run: string;
  readonly resident: string | null;
  readonly at: number;
  readonly part: string | null;
  readonly outcome: UseOutcome;
}

export function outcomeWord(outcome: UseOutcome): Key {
  switch (outcome) {
    case "ok":
      return "usage_outcome_ok";
    case "failed":
      return "usage_outcome_failed";
    case "unknown":
      return "usage_outcome_unknown";
  }
}

export function dayLine(days: readonly DayCount[]): string {
  // wording-ok: a calendar day and a count, spelled the same in every language.
  return days.map((day) => `${day.day} ${String(day.count)}`).join(" · ");
}

// What a look of the uses is given, every word already in the reader's
// language and every moment already on the reader's clock.
export type UsesLook =
  | { readonly kind: "never"; readonly said: string }
  | {
      readonly kind: "used";
      readonly count: string;
      readonly days: string;
      readonly fold: string;
      readonly uses: readonly {
        readonly at: string;
        readonly who: string;
        readonly part: string | undefined;
        readonly outcome: string;
      }[];
    };
