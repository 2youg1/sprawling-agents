// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one row of the mailbox's recent sessions is handed: the room,
// how the session began and how many runs it holds, how long ago its
// last line was written, the link to the room, and the way to fork the
// session from its last turn, with why it cannot be forked when the
// page no longer holds that turn (client/Spec.lean §4-49, 4-14). While
// the branch it started waits for the city, the row says so where it
// says how the session began, and a refusal stays there with `/stop`
// beside it (client D100).

import type { HTMLButtonAttributes } from "svelte/elements";

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import type { Address, SessionLine } from "../../wire";
import type { BranchHere } from "../talk/branching";
import { noteOf, waitWhy } from "../talk/branch_note";
import type { StopKey } from "../talk/branch_note";
import type { EntryLink } from "./entries";

export interface RecentRowLook {
  readonly room: string;
  // How the session began, then how many runs it holds; or, while a
  // branch from this row waits or was refused, what became of it.
  readonly began: string;
  readonly ago: string;
  // Spread on the row's link: the room, the press that puts the mailbox
  // away, and the entry mark j, k and the digits walk.
  readonly link: EntryLink;
  readonly fork: ForkKey;
  readonly stop: StopKey | null;
}

export interface ForkKey {
  readonly name: string;
  // Why the key does nothing, shown as its hint in place of its name.
  readonly why: string | undefined;
  readonly wire: Pick<HTMLButtonAttributes, "type" | "aria-label" | "aria-disabled" | "onclick">;
}

// What the row reads of one session.
export interface Shown {
  readonly room: Address;
  readonly line: SessionLine;
  // Whether the page still holds the session's last turn, where a fork
  // would cut.
  readonly forkable: boolean;
  // What the page's branch is to this row (`talk/branching.ts`).
  readonly branch: BranchHere;
}

export interface RowHands {
  readonly href: string;
  readonly follow: () => void;
  readonly fork: () => void;
  // Cancels the run working in the room now; `null` when none is.
  readonly stop: (() => void) | null;
}

export function rowOf(shown: Shown, lang: Lang, ago: string, hands: RowHands): RecentRowLook {
  const name = say(lang, "mailbox_fork_last");
  const runs = shown.line.runs === 1 ? say(lang, "mailbox_run_one") : fill(say(lang, "mailbox_runs"), { n: String(shown.line.runs) });
  const note = noteOf(shown.branch, shown.room, lang, hands.stop);
  const waiting = waitWhy(shown.branch);
  const why = shown.forkable ? waiting : "mailbox_fork_unheld";
  return {
    room: shown.room,
    began: note?.said ?? `${say(lang, startOf(shown.line))} · ${runs}`,
    ago,
    link: { href: hands.href, onclick: hands.follow, "data-entry": "" },
    fork: {
      name,
      why: why === undefined ? undefined : say(lang, why),
      wire: {
        type: "button",
        "aria-label": name,
        "aria-disabled": why !== undefined,
        onclick: hands.fork,
      },
    },
    stop: note?.stop ?? null,
  };
}

// How a session began, in the words the row says it with.
function startOf(line: SessionLine): Key {
  if ("dispatched" in line.start) return "mailbox_start_dispatched";
  const opened = line.start.opened;
  if (opened.from !== undefined && opened.from !== null) return "mailbox_start_forked";
  return opened.carry === "handoff" ? "mailbox_start_carried" : "mailbox_start_new";
}
