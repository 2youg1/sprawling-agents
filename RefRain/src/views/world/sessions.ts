// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { RunBelief } from "../../core/belief";
import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { roomOf, toFragment } from "../../core/route";
import type { View } from "../../core/route";
import type { Pinning, Stretch } from "../../core/stretches";
import { tailOf } from "../../core/stretches";
import { PIN } from "../../core/tags";
import type { Named } from "../../core/tags";
import { ago, lasted } from "../../core/time";
import type { Address, Origin, RunId, Tag } from "../../wire";
import type { BranchHere } from "../talk/branching";
import { noteOf } from "../talk/branch_note";
import type { BranchNote } from "../talk/branch_note";
import { phaseSaid } from "../runs/phase";
import { called, dispatcherOf } from "../talk/naming";


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
  
  readonly word: string;
  readonly wire: ToggleWire;
}


export interface LinkWire {
  readonly href: string;
  readonly "aria-current": "page" | undefined;
  readonly "aria-busy": boolean;
  readonly onclick: (event: MouseEvent) => void;
}

export interface DisclosureWire {
  readonly type: "button";
  readonly "aria-label": string;
  readonly "aria-expanded": boolean;
  readonly onclick: () => void;
}


export interface PinLook {
  readonly hint: string;
  readonly wire: { readonly role: "img"; readonly "aria-label": string };
}


export interface MenuProps {
  readonly named: Named | null;
  readonly label: string;
  readonly tags: readonly Tag[];
  readonly pinning: Pinning;
  readonly session: { readonly name: string; readonly run: RunId | null };
  
  readonly read: View | null;
}


export interface RowLook {
  readonly key: string;
  readonly depth: number;
  readonly disclosure: DisclosureWire | undefined;
  readonly link: LinkWire;
  
  readonly chosen: boolean;
  
  readonly dot: Dot | undefined;
  readonly title: string;
  
  readonly room: string;
  
  readonly current: boolean;
  readonly pin: PinLook | undefined;
  
  
  readonly when: string | undefined;
  
  
  readonly phase: string | undefined;
  readonly delegated: string | undefined;
  readonly about: string | undefined;
  readonly ranOn: string | undefined;
  readonly tags: readonly Tag[];
  
  
  readonly context: { readonly room: Address; readonly run: RunId } | undefined;
  readonly menu: MenuProps;
  
  readonly note: BranchNote | null;
}

export interface SessionsLook {
  
  readonly filter:
    { readonly wire: FilterWire; readonly toggles: readonly ToggleLook[] } | undefined;
  
  
  readonly narrow: boolean;
  readonly rows: readonly RowLook[];
}


export interface RowContext {
  readonly lang: Lang;
  
  readonly now: number;
  readonly chosen: boolean;
  readonly tags: readonly Tag[];
  readonly pinning: Pinning;
  readonly named: Named | null;
  readonly depth: number;
  readonly children: boolean;
  readonly collapsed: boolean;
  readonly onToggle: () => void;
  readonly branch: BranchHere;
  readonly open: (origin: Origin) => void;
  readonly stop: (() => void) | null;
}

export function rowOf(stretch: Stretch, at: RowContext): RowLook {
  const { lang, now } = at;
  const last = stretch.runs.at(-1);
  const dot = dotOf(stretch);
  const title = titleOf(stretch);
  const told = at.chosen ? undefined : toldOf(stretch, dot, at);
  return {
    key: `${stretch.room}:${String(stretch.line.began)}`,
    depth: at.depth,
    disclosure: at.children
      ? {
          type: "button",
          "aria-label": say(lang, at.collapsed ? "world_expand" : "world_collapse"),
          "aria-expanded": !at.collapsed,
          onclick: at.onToggle,
        }
      : undefined,
    link: {
      href: hrefOf(stretch),
      "aria-current": at.chosen ? "page" : undefined,
      "aria-busy": at.branch.kind === "opening",
      onclick: (event) => {
        const origin = tailOf(stretch);
        if (stretch.current || origin === null) return;
        event.preventDefault();
        at.open(origin);
      },
    },
    chosen: at.chosen,
    dot: at.chosen ? undefined : dot,
    title,
    room: stretch.room,
    current: stretch.current,
    pin: at.pinning === "none" ? undefined : pinOf(at.pinning, lang),
    when: at.chosen && dot !== "run" ? undefined : whenOf(stretch, dot, lang, now),
    phase: told?.phase,
    delegated: told?.delegated,
    about: told?.about,
    ranOn: told?.ranOn,
    tags: at.tags.filter((tag) => tag !== PIN),
    context: told?.context,
    menu: {
      named: at.named,
      label: fill(say(lang, "world_row_name"), {
        room: title,
        when: ago(lang, stretch.line.at, now),
      }),
      tags: at.tags,
      pinning: at.pinning,
      session: { name: stretch.line.name ?? "", run: stretch.current ? (last?.run ?? null) : null },
      read: stretch.current
        ? null
        : { kind: "talk", address: stretch.room, session: stretch.line.began },
    },
    note: noteOf(at.branch, stretch.room, lang, at.stop),
  };
}


type Told = Pick<RowLook, "phase" | "delegated" | "about" | "ranOn" | "context">;

function toldOf(stretch: Stretch, dot: Dot, at: RowContext): Told {
  const { lang, now } = at;
  const last = stretch.runs.at(-1);
  const delegated = delegatedBy(stretch, lang);
  const ranOn = ranOnOf(stretch, lang);
  return {
    phase:
      dot === "run" && last?.doing.kind === "awaiting_reply"
        ? phaseSaid(lang, last.doing, now)
        : undefined,
    delegated: delegated === "" ? undefined : delegated,
    about: aboutOf(stretch, last, lang),
    ranOn: ranOn === "" ? undefined : ranOn,
    context:
      stretch.current && last !== undefined ? { room: stretch.room, run: last.run } : undefined,
  };
}


function pinOf(pinning: Exclude<Pinning, "none">, lang: Lang): PinLook {
  const hint = say(lang, pinning === "mayor" ? "world_pinned_mayor" : "world_pinned_tagged");
  return { hint, wire: { role: "img", "aria-label": hint } };
}

function hrefOf(stretch: Stretch): string {
  return toFragment(
    stretch.current
      ? { kind: "talk", address: stretch.room }
      : { kind: "talk", address: stretch.room, session: stretch.line.began },
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


function aboutOf(stretch: Stretch, last: RunBelief | undefined, lang: Lang): string {
  return stretch.line.preview ?? last?.task ?? say(lang, startOf(stretch));
}

export function titleOf(stretch: Stretch): string {
  const name = stretch.line.name ?? "";
  return name === "" ? roomOf(stretch.room) : name;
}


function ranOnOf(stretch: Stretch, lang: Lang): string {
  const { model, effort, workspace } = stretch.line;
  return [
    model ?? null,
    effort === undefined || effort === null ? null : say(lang, `effort_${effort}`),
    workspace ?? null,
  ]
    .filter((each) => each !== null && each !== "")
    .join(" · ");
}

function startOf(stretch: Stretch): Key {
  const start = stretch.line.start;
  if ("dispatched" in start) return "mailbox_start_dispatched";
  if (start.opened.from !== undefined && start.opened.from !== null) return "mailbox_start_forked";
  return start.opened.carry === "handoff" ? "mailbox_start_carried" : "mailbox_start_new";
}


function delegatedBy(stretch: Stretch, lang: Lang): string {
  const start = stretch.line.start;
  const by = "dispatched" in start ? dispatcherOf(start.dispatched.by) : null;
  return by?.kind === "resident"
    ? fill(say(lang, "talk_dispatched_by"), { who: called(by.address, null, lang) })
    : "";
}


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
