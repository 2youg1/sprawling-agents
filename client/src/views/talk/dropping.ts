// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { bearing } from "../../core/socket";

export type Kept = { readonly kind: "path"; readonly path: string } | Refused;

export interface Refused {
  readonly kind: "refused";
  readonly name: string;
  readonly said: string;
}

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

export function spelled(paths: readonly string[]): string {
  return paths.map((path) => (/\s/.test(path) ? `"${path}"` : path)).join(" ");
}

export async function keep(origin: string, token: string | null, file: File): Promise<Kept> {
  const answer = await fetch(`${origin}/drop?name=${encodeURIComponent(file.name)}`, {
    method: "POST",
    headers: { "content-type": "application/octet-stream", ...bearing(token) },
    body: file,
  }).catch(() => null);
  const said = answer === null ? null : await answer.text().catch(() => null);
  if (answer === null || said === null) return { kind: "refused", name: file.name, said: "" };
  return answer.ok ? { kind: "path", path: said } : { kind: "refused", name: file.name, said };
}

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

export function insertAt(words: string, at: number, paths: readonly string[]): string {
  if (paths.length === 0) return words;
  const before = words.slice(0, at);
  const after = words.slice(at);
  const lead = before === "" || /\s$/.test(before) ? "" : " ";
  const tail = after === "" || /^\s/.test(after) ? "" : " ";
  return `${before}${lead}${spelled(paths)}${tail}${after}`;
}
