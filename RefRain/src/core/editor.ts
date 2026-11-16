// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import { Address } from "../wire";

export type Editor =
  "none" | "vscode" | "vscode-insiders" | "vscodium" | "cursor" | "windsurf" | "zed";

export const EDITORS: readonly Editor[] = [
  "none",
  "vscode",
  "vscode-insiders",
  "vscodium",
  "cursor",
  "windsurf",
  "zed",
];

export interface Opening {
  readonly editor: Editor;

  readonly folder: string;

  readonly path: string;

  readonly line: number;
}

const isAddress = Schema.is(Address);

export function editorLink(at: Opening): string | null {
  const editor = at.editor;
  if (editor === "none" || !isAddress(at.path) || !Number.isSafeInteger(at.line) || at.line < 1) {
    return null;
  }
  const folder = absoluteFolder(at.folder);
  if (folder === null) return null;
  const segments = [...folder.segments, ...at.path.split("/")].map(encodeURIComponent);
  return `${fileUrl(editor, folder.drive)}${segments.join("/")}:${String(at.line)}:1`;
}

function fileUrl(editor: Exclude<Editor, "none">, drive: string): string {
  switch (editor) {
    case "vscode":
    case "vscode-insiders":
    case "vscodium":
    case "cursor":
    case "windsurf":
      return `${editor}://file/${drive}`;
    case "zed":
      return drive === "" ? "zed://file/" : `zed://file//%3F/${drive}`;
  }
}

interface Folder {
  readonly drive: string;
  readonly segments: readonly string[];
}

function absoluteFolder(written: string): Folder | null {
  const forward = written.replaceAll("\\", "/");
  const drive = /^[A-Za-z]:\//.exec(forward)?.[0] ?? "";

  if ((drive === "" && !forward.startsWith("/")) || forward.startsWith("//")) return null;
  const segments = forward
    .slice(drive.length)
    .split("/")
    .filter((each) => each !== "");
  return segments.some((each) => each === "." || each === "..") ? null : { drive, segments };
}

export type Shown = "current" | "past";

export type Reach =
  | { readonly kind: "link"; readonly href: string }
  | { readonly kind: "copy"; readonly text: string };

export function reachOf(at: Opening, shown: Shown): Reach {
  const href = shown === "current" ? editorLink(at) : null;
  return href === null
    ? { kind: "copy", text: `${at.path}:${String(at.line)}` }
    : { kind: "link", href };
}
