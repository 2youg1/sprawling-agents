// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the sessions pane's look is given (`sessions.look.svelte`), and
// how the seat (`sessions.svelte`) reads one session into a row of it
// (client/Spec.lean §7K): its state, its title, the time beside it and
// the lines under it, all in the person's language. The pane decides
// which rows there are and in what order (`core/stretches.ts`); this
// file decides what one row says.

import type { RunBelief } from "../../core/belief";
import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { roomOf, toFragment } from "../../core/route";
import type { Pinning, Stretch } from "../../core/stretches";
import { PIN } from "../../core/tags";
import type { Named } from "../../core/tags";
import { ago, lasted } from "../../core/time";
import type { Address, RunId, Tag } from "../../wire";
import { phaseSaid } from "../runs/phase";
import { called, dispatcherOf } from "../talk/naming";

// A row's state dot: running, waiting for the person, or done. A past
// session is done whatever its last run says: the city goes on only
// with the current one.
export type Dot = "run" | "ask" | "done";

export function dotOf(stretch: Stretch): Dot {
  const last = stretch.runs.at(-1);
  if (!stretch.current || last === undefined) return "done";
  switch (last.doing.kind) {
    case "frozen":
      return "done";
    case "waiting":
      return "ask";
    case "unknown":
    case "thinking":
    case "calling":
    case "awaiting_reply":
      return "run";
  }
}

// The bag spread on the tag filter's group, and on each of its toggles.
export interface FilterWire {
  readonly role: "group";
  readonly "aria-label": string;
}
export interface ToggleWire {
  readonly type: "button";
  readonly "aria-pressed": boolean;
  readonly onclick: () => void;
}

export interface ToggleLook {
  readonly key: string;
  // The tag, or "all"; already in the person's language.
  readonly word: string;
  readonly wire: ToggleWire;
}

// The bag spread on a row's link: where it goes, and whether its
// session is the one in main.
export interface LinkWire {
  readonly href: string;
  readonly "aria-current": "page" | undefined;
}

// The pin a pinned row carries: a mark with a name, and the hint that
// says who pinned it.
export interface PinLook {
  readonly hint: string;
  readonly wire: { readonly role: "img"; readonly "aria-label": string };
}

// What a row's menu seat (`session_menu.svelte`) is handed.
export interface MenuProps {
  readonly named: Named | null;
  readonly label: string;
  readonly tags: readonly Tag[];
  readonly pinning: Pinning;
  readonly session: { readonly name: string; readonly run: RunId | null };
}

export interface RowLook {
  readonly key: string;
  readonly link: LinkWire;
  // Whether this row's session is the one in main.
  readonly chosen: boolean;
  readonly dot: Dot;
  readonly title: string;
  // The room's address, which the title stands for when it is a name.
  readonly room: string;
  // Whether this is its room's current session; a past one is quieter.
  readonly current: boolean;
  readonly pin: PinLook | undefined;
  // How long it has run, that it waits, or how long ago it ended.
  readonly when: string;
  // The lines under the title, each absent when it has nothing to say:
  // the phase of a run waiting on a reply, who handed the work down,
  // what the session is about, and what it ran on.
  readonly phase: string | undefined;
  readonly delegated: string | undefined;
  readonly about: string;
  readonly ranOn: string | undefined;
  readonly tags: readonly Tag[];
  // The run whose context the row's bar reads (`context_bar.svelte`),
  // for a current session that holds one.
  readonly context: { readonly room: Address; readonly run: RunId } | undefined;
  readonly menu: MenuProps;
}

export interface GroupLook {
  readonly key: string;
  readonly heading: string;
  readonly rows: readonly RowLook[];
}

export interface SessionsLook {
  // Absent while no row carries a tag.
  readonly filter: { readonly wire: FilterWire; readonly toggles: readonly ToggleLook[] } | undefined;
  // Whether the pane is as narrow as the right pane leaves it, which
  // drops the second line and the time.
  readonly narrow: boolean;
  readonly groups: readonly GroupLook[];
}

// What a row is read from beyond its session.
export interface RowContext {
  readonly lang: Lang;
  // The page's clock, which a running row's time reads.
  readonly now: number;
  readonly chosen: boolean;
  readonly tags: readonly Tag[];
  readonly pinning: Pinning;
  readonly named: Named | null;
}

export function rowOf(stretch: Stretch, at: RowContext): RowLook {
  const { lang, now } = at;
  const last = stretch.runs.at(-1);
  const dot = dotOf(stretch);
  const title = titleOf(stretch);
  const delegated = delegatedBy(stretch, lang);
  const ranOn = ranOnOf(stretch, lang);
  return {
    key: `${stretch.room}:${String(stretch.line.began)}`,
    link: { href: hrefOf(stretch), "aria-current": at.chosen ? "page" : undefined },
    chosen: at.chosen,
    dot,
    title,
    room: stretch.room,
    current: stretch.current,
    pin: at.pinning === "none" ? undefined : pinOf(at.pinning, lang),
    when: whenOf(stretch, dot, lang, now),
    phase: dot === "run" && last?.doing.kind === "awaiting_reply" ? phaseSaid(lang, last.doing, now) : undefined,
    delegated: delegated === "" ? undefined : delegated,
    about: aboutOf(stretch, last, lang),
    ranOn: ranOn === "" ? undefined : ranOn,
    tags: at.tags.filter((tag) => tag !== PIN),
    context: stretch.current && last !== undefined ? { room: stretch.room, run: last.run } : undefined,
    menu: {
      named: at.named,
      label: fill(say(lang, "world_row_name"), { room: title, when: ago(lang, stretch.line.at, now) }),
      tags: at.tags,
      pinning: at.pinning,
      session: { name: stretch.line.name ?? "", run: stretch.current ? (last?.run ?? null) : null },
    },
  };
}

// Who pinned it, said both ways: the Mayor's current session is pinned
// by being current, any other by the person's tag.
function pinOf(pinning: Exclude<Pinning, "none">, lang: Lang): PinLook {
  const hint = say(lang, pinning === "mayor" ? "world_pinned_mayor" : "world_pinned_tagged");
  return { hint, wire: { role: "img", "aria-label": hint } };
}

function hrefOf(stretch: Stretch): string {
  return toFragment(
    stretch.current ? { kind: "talk", address: stretch.room } : { kind: "talk", address: stretch.room, session: stretch.line.began },
  );
}

function whenOf(stretch: Stretch, dot: Dot, lang: Lang, now: number): string {
  switch (dot) {
    case "run": {
      const started = stretch.runs.at(-1)?.started ?? null;
      return started === null ? "" : lasted(now - started);
    }
    case "ask":
      return say(lang, "world_waiting");
    case "done":
      return ago(lang, stretch.line.at, now);
  }
}

// What the second line says: the start of the session's last reply,
// else the task of its last run, else how the session began when this
// page holds none of its runs.
function aboutOf(stretch: Stretch, last: RunBelief | undefined, lang: Lang): string {
  return stretch.line.preview ?? last?.task ?? say(lang, startOf(stretch));
}

export function titleOf(stretch: Stretch): string {
  const name = stretch.line.name ?? "";
  return name === "" ? roomOf(stretch.room) : name;
}

// What the session ran on: model, effort and workspace, each left out
// when the city does not say it.
function ranOnOf(stretch: Stretch, lang: Lang): string {
  const { model, effort, workspace } = stretch.line;
  return [model ?? null, effort === undefined || effort === null ? null : say(lang, `effort_${effort}`), workspace ?? null]
    .filter((each) => each !== null && each !== "")
    .join(" · ");
}

function startOf(stretch: Stretch): Key {
  const start = stretch.line.start;
  if ("dispatched" in start) return "mailbox_start_dispatched";
  if (start.opened.from !== undefined && start.opened.from !== null) return "mailbox_start_forked";
  return start.opened.carry === "handoff" ? "mailbox_start_carried" : "mailbox_start_new";
}

// The resident a session's work was handed down by, as the session
// line spells it (`SessionStart::Dispatched { by }`), in the words the
// thread's own opening uses; empty for a session the User or the city
// began, which the second line already says.
function delegatedBy(stretch: Stretch, lang: Lang): string {
  const start = stretch.line.start;
  const by = "dispatched" in start ? dispatcherOf(start.dispatched.by) : null;
  return by?.kind === "resident" ? fill(say(lang, "talk_dispatched_by"), { who: called(by.address, null, lang) }) : "";
}

// The tag filter: "all" first, then each tag a listed row carries; one
// is held at a time, and pressing the held one again goes back to all.
export function filterOf(
  offered: readonly Tag[],
  by: Tag | null,
  words: { readonly label: string; readonly all: string },
  pick: (tag: Tag | null) => void,
): SessionsLook["filter"] {
  if (offered.length === 0) return undefined;
  return {
    wire: { role: "group", "aria-label": words.label },
    toggles: [null, ...offered].map((tag) => ({
      key: tag ?? "",
      word: tag ?? words.all,
      wire: {
        type: "button",
        "aria-pressed": by === tag,
        onclick: () => {
          pick(by === tag ? null : tag);
        },
      },
    })),
  };
}
