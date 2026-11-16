// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


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
  
  
  readonly began: string;
  readonly ago: string;
  
  
  readonly link: EntryLink;
  readonly fork: ForkKey;
  readonly stop: StopKey | null;
}

export interface ForkKey {
  readonly name: string;
  
  readonly why: string | undefined;
  readonly wire: Pick<HTMLButtonAttributes, "type" | "aria-label" | "aria-disabled" | "onclick">;
}


export interface Shown {
  readonly room: Address;
  readonly line: SessionLine;
  
  
  readonly forkable: boolean;
  
  readonly branch: BranchHere;
}

export interface RowHands {
  readonly href: string;
  readonly follow: () => void;
  readonly fork: () => void;
  
  readonly stop: (() => void) | null;
}

export function rowOf(shown: Shown, lang: Lang, ago: string, hands: RowHands): RecentRowLook {
  const name = say(lang, "mailbox_fork_last");
  const runs =
    shown.line.runs === 1
      ? say(lang, "mailbox_run_one")
      : fill(say(lang, "mailbox_runs"), { n: String(shown.line.runs) });
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


function startOf(line: SessionLine): Key {
  if ("dispatched" in line.start) return "mailbox_start_dispatched";
  const opened = line.start.opened;
  if (opened.from !== undefined && opened.from !== null) return "mailbox_start_forked";
  return opened.carry === "handoff" ? "mailbox_start_carried" : "mailbox_start_new";
}
