// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { editorLink } from "../../core/editor";
import type { Opening } from "../../core/editor";
import { say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { lineOf, reverseOf } from "./hunk";
import type { Hunk, Sign, Touched } from "./trace";

export interface PressWire {
  readonly type: "button";
  readonly onclick: () => void;
}

export interface OpenWire {
  readonly href: string;
}

export type ActionLook =
  | {
      readonly key: "comment" | "revert";
      readonly label: string;
      readonly kind: "press";
      readonly wire: PressWire;
    }
  | {
      readonly key: "open";
      readonly label: string;
      readonly kind: "open";
      readonly wire: OpenWire;
    };

export interface LineLook {
  readonly sign: Sign;
  readonly old: string;
  readonly new: string;
  readonly text: string;
}

export interface HunkLook {
  readonly actions: readonly ActionLook[];
  readonly lines: readonly LineLook[];
}

export interface FileLook {
  readonly path: string;
  readonly how: string;
  readonly hunks: readonly HunkLook[];
}

export interface CodeColumnLook {
  readonly empty: string | undefined;
  readonly files: readonly FileLook[];
}

export interface ColumnState {
  readonly files: readonly Touched[];
  readonly live: boolean;
  readonly editor: Pick<Opening, "editor" | "folder">;
}

export interface Hands {
  readonly draft: (text: string) => void;
  readonly steer: (text: string) => void;
}

export function lookOf(state: ColumnState, lang: Lang, hands: Hands): CodeColumnLook {
  const actionsOf = (path: string, hunk: Hunk): ActionLook[] => {
    const presses: ActionLook[] = state.live
      ? [
          {
            key: "comment",
            label: say(lang, "mon_comment"),
            kind: "press",
            wire: {
              type: "button",
              onclick: () => {
                hands.draft(steerOf("mon_comment_draft", lang, path, hunk));
              },
            },
          },
          {
            key: "revert",
            label: say(lang, "mon_revert"),
            kind: "press",
            wire: {
              type: "button",
              onclick: () => {
                hands.steer(steerOf("mon_revert_steer", lang, path, hunk));
              },
            },
          },
        ]
      : [];
    const opening = editorLink({ ...state.editor, path, line: lineOf(hunk) });
    return opening === null
      ? presses
      : [
          ...presses,
          { key: "open", label: say(lang, "setup_editor"), kind: "open", wire: { href: opening } },
        ];
  };
  return {
    empty: state.files.length === 0 ? say(lang, "mon_nothing_changed") : undefined,
    files: state.files.map((file) => ({
      path: file.path,
      how: say(lang, file.how === "added" ? "change_added" : "change_modified"),
      hunks: file.hunks.map((hunk) => ({
        actions: actionsOf(file.path, hunk),
        lines: hunk.lines.map((line) => ({
          sign: line.sign,
          old: line.old?.toString() ?? "",
          new: line.new?.toString() ?? "",
          text: line.text,
        })),
      })),
    })),
  };
}

export function steerOf(
  key: "mon_comment_draft" | "mon_revert_steer",
  lang: Lang,
  path: string,
  hunk: Hunk,
): string {
  const slots: Record<string, string> = { path, line: String(lineOf(hunk)), ...reverseOf(hunk) };
  return say(lang, key).replace(
    /\{(path|line|now|was|fence)\}/g,
    (spelled, name: string) => slots[name] ?? spelled,
  );
}
