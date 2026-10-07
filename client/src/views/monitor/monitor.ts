// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the monitor's controls do, decided without a DOM: which keys
// pause following and which one takes it up again, and the whole value
// the monitor's look draws (`MonitorLook`), with every word translated
// and every control's role, `aria-*` value and handler inside a wire
// bag the look spreads unchanged (client D95). The seat
// (`monitor.svelte`) holds the two states and the two panes; this file
// holds no state of its own.

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";
import type { Snippet } from "svelte";

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { count } from "../../core/time";

// Following lands each pane on whatever the agent touched last;
// paused leaves both where the person put them.
export type Following = "following" | "paused";

// Open draws the terminal beside the code; folded draws the column of
// changed files in its place.
export type Pane = "open" | "folded";

// The keys that scroll a focused pane. A person pressing one is
// reading, so it pauses following, as a wheel turn or a touch drag does.
const SCROLLING = new Set(["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End", " "]);

// What a pane's key handler reads from the event.
export type KeyPress = Pick<KeyboardEvent, "key">;

// What the page-wide `F` handler reads from the event: the key, the
// three modifiers, and whether the person is typing into a field, which
// the seat answers from the event's target.
export interface ResumePress {
  readonly key: string;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
  readonly altKey: boolean;
  readonly typing: boolean;
}

export function scrolls(press: KeyPress): boolean {
  return SCROLLING.has(press.key);
}

// `F` takes following up again, heard page-wide because it only means
// something while the monitor is on the page; a modified `F` belongs to
// the browser or the shell, and an `F` typed into a field is a letter.
export function resumes(press: ResumePress): boolean {
  return press.key.toLowerCase() === "f" && !press.typing && !press.ctrlKey && !press.metaKey && !press.altKey;
}

// The bag spread on the follow toggle.
export interface FollowWire {
  readonly type: "button";
  readonly "aria-pressed": boolean;
  readonly onclick: () => void;
}

// The bag spread on the terminal's fold toggle.
export interface FoldWire {
  readonly type: "button";
  readonly "aria-expanded": boolean;
  readonly onclick: () => void;
}

// The bag spread on one file of the folded column.
export interface JumpWire {
  readonly type: "button";
  readonly onclick: () => void;
}

// The bag spread on a scrolling pane. The pane takes focus so the
// keyboard can scroll it, and its symbol key is a Svelte attachment
// that hands the seat the element it scrolls (`createAttachmentKey`),
// so the look holds no element reference of its own.
export interface PaneWire {
  readonly role: "region";
  readonly "aria-label": string;
  readonly tabindex: 0;
  readonly onwheel: () => void;
  readonly ontouchmove: () => void;
  readonly onkeydown: (press: KeyPress) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

export interface JumpLook {
  // Unique among the run's files, so it also keys the `#each`.
  readonly path: string;
  readonly wire: JumpWire;
}

// Everything the look is given except the two panes' contents, which
// the seat draws and hands over as snippets.
export interface MonitorWiring {
  readonly label: string;
  readonly heading: string;
  readonly tally: string;
  readonly follow: { readonly label: string; readonly held: boolean; readonly wire: FollowWire };
  readonly fold: { readonly label: string; readonly wire: FoldWire };
  readonly terminal: Pane;
  // The folded column of changed files; absent while the terminal is open.
  readonly index: { readonly label: string; readonly files: readonly JumpLook[] } | undefined;
  readonly code: PaneWire;
  // The terminal pane; absent while it is folded.
  readonly record: PaneWire | undefined;
}

export interface MonitorLook extends MonitorWiring {
  readonly column: Snippet;
  readonly printed: Snippet;
}

export interface MonitorState {
  readonly following: Following;
  readonly terminal: Pane;
  readonly paths: readonly string[];
}

// What only the seat can do, because only it holds the state and the
// panes. `holdCode` and `holdRecord` are the same attachment on every
// call, so a redraw of the look does not let go of a pane and take it
// again.
export interface Hands {
  readonly follow: (next: Following) => void;
  readonly fold: (next: Pane) => void;
  readonly jump: (path: string) => void;
  readonly holdCode: Attachment<HTMLElement>;
  readonly holdRecord: Attachment<HTMLElement>;
}

// One key for every pane's attachment; Svelte keeps one attachment per
// element under each symbol key.
const HOLD = createAttachmentKey();

export function lookOf(state: MonitorState, lang: Lang, hands: Hands): MonitorWiring {
  const paneOf = (label: string, hold: Attachment<HTMLElement>): PaneWire => ({
    role: "region",
    "aria-label": label,
    tabindex: 0,
    onwheel: () => {
      hands.follow("paused");
    },
    ontouchmove: () => {
      hands.follow("paused");
    },
    onkeydown: (press) => {
      if (scrolls(press)) hands.follow("paused");
    },
    [HOLD]: hold,
  });
  const following = state.following === "following";
  const open = state.terminal === "open";
  return {
    label: say(lang, "mon_monitor"),
    heading: say(lang, "mon_code"),
    tally: state.paths.length === 1 ? say(lang, "mon_files_one") : fill(say(lang, "mon_files_n"), { n: count(state.paths.length) }),
    follow: {
      label: say(lang, following ? "mon_following" : "mon_paused"),
      held: following,
      wire: {
        type: "button",
        "aria-pressed": following,
        onclick: () => {
          hands.follow(following ? "paused" : "following");
        },
      },
    },
    fold: {
      label: say(lang, open ? "mon_terminal_fold" : "mon_terminal_open"),
      wire: {
        type: "button",
        "aria-expanded": open,
        onclick: () => {
          hands.fold(open ? "folded" : "open");
        },
      },
    },
    terminal: state.terminal,
    index: open
      ? undefined
      : {
          label: say(lang, "mon_files"),
          files: state.paths.map((path) => ({
            path,
            wire: {
              type: "button",
              onclick: () => {
                hands.jump(path);
              },
            },
          })),
        },
    code: paneOf(say(lang, "mon_code"), hands.holdCode),
    record: open ? paneOf(say(lang, "mon_terminal"), hands.holdRecord) : undefined,
  };
}
