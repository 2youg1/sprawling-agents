// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Files dragged onto the box become paths in it. A browser does not tell
// a page where a dropped file lives: a drag from a file manager that
// carries `file://` URIs names the local paths itself, and those are
// used as they are; any other drop sends each file to the city over the
// page's own origin (`POST /drop`, `crates/wire/Spec.lean` §8-49), which keeps it
// and answers with the absolute path it is at.

import { bearing } from "../../core/socket";

// What one file came to.
export type Kept =
  | { readonly kind: "path"; readonly path: string }
  | { readonly kind: "refused"; readonly name: string; readonly said: string };

// The local paths a `text/uri-list` names. Lines starting `#` are
// comments in that format; a URI that is not `file:` names nothing on
// this machine and is left out. A Windows drive path loses the slash
// the URI puts before its letter.
export function localPaths(uriList: string): readonly string[] {
  return uriList
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.startsWith("file://"))
    .map((line) => {
      const url = new URL(line);
      const path = decodeURIComponent(url.pathname);
      const host = url.host === "" ? "" : `//${url.host}`;
      return /^\/[A-Za-z]:\//.test(path) ? path.slice(1) : `${host}${path}`;
    });
}

// Paths as the box writes them: one space between, and a path with a
// space in it quoted so it stays one path to whoever reads it.
export function spelled(paths: readonly string[]): string {
  return paths.map((path) => (/\s/.test(path) ? `"${path}"` : path)).join(" ");
}

// Sends one file to the city and reads back where it was kept.
export async function keep(origin: string, token: string | null, file: File): Promise<Kept> {
  const answer = await fetch(`${origin}/drop?name=${encodeURIComponent(file.name)}`, {
    method: "POST",
    headers: { "content-type": "application/octet-stream", ...bearing(token) },
    body: file,
  }).catch(() => null);
  if (answer === null) return { kind: "refused", name: file.name, said: "" };
  const said = await answer.text();
  return answer.ok ? { kind: "path", path: said } : { kind: "refused", name: file.name, said };
}

// What a drop carries: nothing this box takes, the paths a file manager
// named, or files on their way to the city.
export type Drop =
  | { readonly kind: "nothing" }
  | { readonly kind: "named"; readonly paths: readonly string[] }
  | { readonly kind: "sent"; readonly kept: Promise<readonly Kept[]> };

export function dropped(carried: DataTransfer | null, origin: string, token: string | null): Drop {
  if (carried === null) return { kind: "nothing" };
  const paths = localPaths(carried.getData("text/uri-list"));
  if (paths.length > 0) return { kind: "named", paths };
  const files = [...carried.files];
  if (files.length === 0) return { kind: "nothing" };
  return { kind: "sent", kept: Promise.all(files.map((file) => keep(origin, token, file))) };
}

// The words with the paths placed at `at`, with a space on either side
// that the words around them did not already have.
export function insertAt(words: string, at: number, paths: readonly string[]): string {
  if (paths.length === 0) return words;
  const before = words.slice(0, at);
  const after = words.slice(at);
  const lead = before === "" || /\s$/.test(before) ? "" : " ";
  const tail = after === "" || /^\s/.test(after) ? "" : " ";
  return `${before}${lead}${spelled(paths)}${tail}${after}`;
}
