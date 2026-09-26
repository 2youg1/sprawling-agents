// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one spelling of "open this file at this line in my editor".
//
// The page hands the browser a link and the browser hands it to the
// editor this machine registered for the scheme; the city starts
// nothing. Only editors whose own documentation states a file-and-line
// URL are offered (client-SPEC 4-39 cites each one), because a guessed
// scheme fails silently: the browser asks for an application nobody
// installed, or opens the file at its first line.
//
// **Only a path inside the city gets a link.** The path is judged by
// the generated `Address` grammar, which is the city's own rule for a
// relative path and admits no `..`, no drive, no backslash; the folder
// is the person's own entry and must be absolute with no `.` or `..`
// segment. Anything else answers `null` and the page draws no link.

import { Schema } from "effect";

import { Address } from "../wire";

// `none` is the posture a browser starts in: no link is drawn until the
// person names the editor this machine has.
export type Editor = "none" | "vscode" | "vscode-insiders";

// Every value the settings selector offers, in the order it is drawn,
// and the list a stored word is read back through.
export const EDITORS: readonly Editor[] = ["none", "vscode", "vscode-insiders"];

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

// `vscode://file/{full path to file}:line:column`, the form VS Code
// documents; the Insiders build takes the same form under its own
// scheme. The column is always the first, because a line is what the
// monitor knows.
export function editorLink(at: Opening): string | null {
  if (at.editor === "none" || !isAddress(at.path) || !Number.isSafeInteger(at.line) || at.line < 1) {
    return null;
  }
  const folder = absoluteFolder(at.folder);
  if (folder === null) return null;
  const segments = [...folder.segments, ...at.path.split("/")].map(encodeURIComponent);
  return `${at.editor}://file/${folder.drive}${segments.join("/")}:${String(at.line)}:1`;
}

interface Folder {
  // `C:/` on Windows, empty where the root is `/`.
  readonly drive: string;
  readonly segments: readonly string[];
}

function absoluteFolder(written: string): Folder | null {
  const forward = written.replaceAll("\\", "/");
  const drive = /^[A-Za-z]:\//.exec(forward)?.[0] ?? "";
  if (drive === "" && !forward.startsWith("/")) return null;
  const segments = forward
    .slice(drive.length)
    .split("/")
    .filter((each) => each !== "");
  return segments.some((each) => each === "." || each === "..") ? null : { drive, segments };
}
