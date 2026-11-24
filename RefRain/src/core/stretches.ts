// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { RunBelief } from "./belief";
import { MAYOR } from "./route";
import { PIN } from "./tags";
import type { Address, Origin, Seq, SessionLine, SessionsAnswer, Tag } from "../wire";

export interface Stretch {
  readonly room: Address;
  readonly line: SessionLine;
  readonly current: boolean;
  readonly runs: readonly RunBelief[];
}

export function stretchesOf(
  answers: readonly SessionsAnswer[],
  runsIn: (room: Address) => readonly RunBelief[],
): Stretch[] {
  const all = answers.flatMap((answer) => {
    const runs = runsIn(answer.room);
    return answer.sessions.map((line, at): Stretch => {
      const next = answer.sessions[at - 1];
      return {
        room: answer.room,
        line,
        current: at === 0,
        runs: runs.filter((run) =>
          run.sessionBegan !== undefined
            ? run.sessionBegan === line.began
            : run.lastSeq >= line.began && (next === undefined || run.lastSeq < next.began),
        ),
      };
    });
  });
  return all.sort((a, b) => b.line.at - a.line.at);
}

export function lineIn(
  answer: SessionsAnswer | undefined,
  asked: Seq | undefined,
): SessionLine | null {
  const lines = answer?.sessions ?? [];
  return (asked === undefined ? lines[0] : lines.find((line) => line.began === asked)) ?? null;
}

export type Pinning = "mayor" | "tagged" | "none";

export function pinningOf(stretch: Stretch, tags: readonly Tag[]): Pinning {
  if (stretch.room === MAYOR && stretch.current) return "mayor";
  return tags.includes(PIN) ? "tagged" : "none";
}

export function tailOf(stretch: Stretch): Origin | null {
  return stretch.line.tail ?? null;
}
