// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one spelling of "open this file at this line in my editor".
//
// The page hands the browser a link and the browser hands it to the
// editor this machine registered for the scheme; the city starts
// nothing. Only editors whose own documentation or source code reads a
// file-and-line URL are offered (client-SPEC 4-39 cites each one),
// because a guessed scheme fails silently: the browser asks for an
// application nobody installed, or opens the file at its first line.
//
// **Only a path inside the city gets a link.** The path is judged by
// the generated `Address` grammar, which is the city's own rule for a
// relative path and admits no `..`, no drive, no backslash; the folder
// is the person's own entry and must be absolute, not a UNC share,
// with no `.` or `..` segment. Anything else answers `null` and the
// page draws no link. The check is lexical, on the recorded path: a
// link on disk that leads outside the city is not followed, because
// the browser cannot see the disk and the city starts nothing.

import { Schema } from "effect";

import { Address } from "../wire";

// `none` is the posture a browser starts in: no link is drawn until the
// person names the editor this machine has.
export type Editor = "none" | "vscode" | "vscode-insiders" | "vscodium" | "cursor" | "windsurf" | "zed";

// Every value the settings selector offers, in the order it is drawn,
// and the list a stored word is read back through.
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
  // The city's folder on this machine, as the person wrote it: either
  // separator, an optional trailing one.
  readonly folder: string;
  // Relative to the city, `/`-separated, as the ledger records it.
  readonly path: string;
  // One-based, as an editor counts.
  readonly line: number;
}

const isAddress = Schema.is(Address);

// `{scheme}://file/{full path to file}:line:column`. The column is
// always the first, because a line is what the monitor knows.
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

// Where each editor's file handler starts reading the path.
//
// VS Code opens `{scheme}://file/{path}` for whatever scheme the build
// registers, so the Insiders build, VSCodium, Cursor and Windsurf take
// the same form under their own scheme. Zed strips `zed://file` and
// opens the rest as a path, which on Windows leaves `/C:/...`, a name
// Windows refuses; `//?/C:/...` is the device form of the same path and
// opens, with the `?` escaped so the browser does not read a query.
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
  // `C:/` on Windows, empty where the root is `/`.
  readonly drive: string;
  readonly segments: readonly string[];
}

function absoluteFolder(written: string): Folder | null {
  const forward = written.replaceAll("\\", "/");
  const drive = /^[A-Za-z]:\//.exec(forward)?.[0] ?? "";
  // `//host/share` is a UNC path: its host is not a folder, and read as
  // one it would point the link at a different file.
  if ((drive === "" && !forward.startsWith("/")) || forward.startsWith("//")) return null;
  const segments = forward
    .slice(drive.length)
    .split("/")
    .filter((each) => each !== "");
  return segments.some((each) => each === "." || each === "..") ? null : { drive, segments };
}
