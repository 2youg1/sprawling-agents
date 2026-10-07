// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The whole value the code column's look draws (`CodeColumnLook`):
// every file the run changed, its hunks, and the person's actions on
// each hunk, with the words translated and each action's handler or
// address inside a wire bag (client D95). Each action goes through a
// door the city already has: a comment is a steer to this run, drafted
// and left for the person to finish; a revert is a steer asking the
// agent to take the hunk back with its edit tool, which fences the
// write by the file's version and records it in the ledger; and opening
// is a link the browser hands to the editor the person chose, offered
// only when `editorLink` answers one.

import { editorLink } from "../../core/editor";
import type { Opening } from "../../core/editor";
import { say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { lineOf, reverseOf } from "./hunk";
import type { Hunk, Sign, Touched } from "./trace";

// The bag spread on a press: a comment or a revert.
export interface PressWire {
  readonly type: "button";
  readonly onclick: () => void;
}

// The bag spread on the link that opens the hunk in an editor.
export interface OpenWire {
  readonly href: string;
}

export type ActionLook =
  | { readonly key: "comment" | "revert"; readonly label: string; readonly kind: "press"; readonly wire: PressWire }
  | { readonly key: "open"; readonly label: string; readonly kind: "open"; readonly wire: OpenWire };

export interface LineLook {
  readonly sign: Sign;
  // The line's number on the side it exists on, `""` on the other.
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
  // Whether the run added the file or changed it, translated.
  readonly how: string;
  readonly hunks: readonly HunkLook[];
}

export interface CodeColumnLook {
  // The sentence that stands in for an empty column; absent when the
  // run changed something.
  readonly empty: string | undefined;
  readonly files: readonly FileLook[];
}

export interface ColumnState {
  readonly files: readonly Touched[];
  // A finished run refuses a steer and draws no composer, so comment
  // and revert are offered only while it is live; opening the file in
  // an editor needs no run and stays.
  readonly live: boolean;
  readonly editor: Pick<Opening, "editor" | "folder">;
}

export interface Hands {
  // Puts a steer in front of the person to finish and send.
  readonly draft: (text: string) => void;
  // Sends a steer to the run as it stands.
  readonly steer: (text: string) => void;
}

export function lookOf(state: ColumnState, lang: Lang, hands: Hands): CodeColumnLook {
  const actionsOf = (path: string, hunk: Hunk): ActionLook[] => {
    const presses: ActionLook[] = state.live
      ? [
          { key: "comment", label: say(lang, "mon_comment"), kind: "press", wire: { type: "button", onclick: () => { hands.draft(steerOf("mon_comment_draft", lang, path, hunk)); } } },
          { key: "revert", label: say(lang, "mon_revert"), kind: "press", wire: { type: "button", onclick: () => { hands.steer(steerOf("mon_revert_steer", lang, path, hunk)); } } },
        ]
      : [];
    const opening = editorLink({ ...state.editor, path, line: lineOf(hunk) });
    return opening === null ? presses : [...presses, { key: "open", label: say(lang, "setup_editor"), kind: "open", wire: { href: opening } }];
  };
  return {
    empty: state.files.length === 0 ? say(lang, "mon_nothing_changed") : undefined,
    files: state.files.map((file) => ({
      path: file.path,
      how: say(lang, file.how === "added" ? "change_added" : "change_modified"),
      hunks: file.hunks.map((hunk) => ({
        actions: actionsOf(file.path, hunk),
        lines: hunk.lines.map((line) => ({ sign: line.sign, old: line.old?.toString() ?? "", new: line.new?.toString() ?? "", text: line.text })),
      })),
    })),
  };
}

// The steer a comment or a revert carries, filled in one pass with a
// function, so a hunk's own text is never read as a placeholder or as a
// `$&` replacement pattern.
export function steerOf(key: "mon_comment_draft" | "mon_revert_steer", lang: Lang, path: string, hunk: Hunk): string {
  const slots: Record<string, string> = { path, line: String(lineOf(hunk)), ...reverseOf(hunk) };
  return say(lang, key).replace(/\{(path|line|now|was|fence)\}/g, (spelled, name: string) => slots[name] ?? spelled);
}
