// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";
import type { Snippet } from "svelte";

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { count } from "../../core/time";

export type Following = "following" | "paused";

export type Pane = "open" | "folded";

const SCROLLING = new Set(["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End", " "]);

export type KeyPress = Pick<KeyboardEvent, "key">;

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

export function resumes(press: ResumePress): boolean {
  return (
    press.key.toLowerCase() === "f" &&
    !press.typing &&
    !press.ctrlKey &&
    !press.metaKey &&
    !press.altKey
  );
}

export interface FollowWire {
  readonly type: "button";
  readonly "aria-pressed": boolean;
  readonly onclick: () => void;
}

export interface FoldWire {
  readonly type: "button";
  readonly "aria-expanded": boolean;
  readonly onclick: () => void;
}

export interface JumpWire {
  readonly type: "button";
  readonly onclick: () => void;
}

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
  readonly path: string;
  readonly wire: JumpWire;
}

export interface MonitorWiring {
  readonly label: string;
  readonly heading: string;
  readonly tally: string;
  readonly follow: { readonly label: string; readonly held: boolean; readonly wire: FollowWire };
  readonly fold: { readonly label: string; readonly wire: FoldWire };
  readonly terminal: Pane;
  readonly index: { readonly label: string; readonly files: readonly JumpLook[] } | undefined;
  readonly code: PaneWire;
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

export interface Hands {
  readonly follow: (next: Following) => void;
  readonly fold: (next: Pane) => void;
  readonly jump: (path: string) => void;
  readonly holdCode: Attachment<HTMLElement>;
  readonly holdRecord: Attachment<HTMLElement>;
}

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
    tally:
      state.paths.length === 1
        ? say(lang, "mon_files_one")
        : fill(say(lang, "mon_files_n"), { n: count(state.paths.length) }),
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
