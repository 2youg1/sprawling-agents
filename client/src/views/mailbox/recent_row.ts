// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one row of the mailbox's recent sessions is handed: the room,
// how the session began and how many runs it holds, how long ago its
// last line was written, the link to the room, and the way to fork the
// session from its last turn, with why it cannot be forked when the
// page no longer holds that turn (client/Spec.lean §4-49, 4-14).

import type { HTMLButtonAttributes } from "svelte/elements";

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import type { SessionLine } from "../../wire";
import type { EntryLink } from "./entries";

export interface RecentRowLook {
  readonly room: string;
  // How the session began, then how many runs it holds.
  readonly began: string;
  readonly ago: string;
  // Spread on the row's link: the room, the press that puts the mailbox
  // away, and the entry mark j, k and the digits walk.
  readonly link: EntryLink;
  readonly fork: ForkKey;
}

export interface ForkKey {
  readonly name: string;
  // Why the key does nothing, shown as its hint in place of its name.
  readonly why: string | undefined;
  readonly wire: Pick<HTMLButtonAttributes, "type" | "aria-label" | "aria-disabled" | "onclick">;
}

// What the row reads of one session.
export interface Shown {
  readonly room: string;
  readonly line: SessionLine;
  // Whether the page still holds the session's last turn, where a fork
  // would cut.
  readonly forkable: boolean;
}

export interface RowHands {
  readonly href: string;
  readonly follow: () => void;
  readonly fork: () => void;
}

export function rowOf(shown: Shown, lang: Lang, ago: string, hands: RowHands): RecentRowLook {
  const name = say(lang, "mailbox_fork_last");
  const runs = shown.line.runs === 1 ? say(lang, "mailbox_run_one") : fill(say(lang, "mailbox_runs"), { n: String(shown.line.runs) });
  return {
    room: shown.room,
    began: `${say(lang, startOf(shown.line))} · ${runs}`,
    ago,
    link: { href: hands.href, onclick: hands.follow, "data-entry": "" },
    fork: {
      name,
      why: shown.forkable ? undefined : say(lang, "mailbox_fork_unheld"),
      wire: {
        type: "button",
        "aria-label": name,
        "aria-disabled": !shown.forkable,
        onclick: hands.fork,
      },
    },
  };
}

// How a session began, in the words the row says it with.
function startOf(line: SessionLine): Key {
  if ("dispatched" in line.start) return "mailbox_start_dispatched";
  const opened = line.start.opened;
  if (opened.from !== undefined && opened.from !== null) return "mailbox_start_forked";
  return opened.carry === "handoff" ? "mailbox_start_carried" : "mailbox_start_new";
}
