// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Key } from "../../core/lang";


export type Offered =
  | { readonly kind: "blank" }
  | { readonly kind: "refused"; readonly why: Key }
  | { readonly kind: "path"; readonly path: string };


export function offered(written: string): Offered {
  const text = written.trim();
  if (text === "") return { kind: "blank" };
  if (text === "~" || text === "~/") return { kind: "refused", why: "skill_mount_home" };
  if (text.startsWith("~/") || text.startsWith("~\\")) return { kind: "path", path: text };
  const absolute = text.startsWith("/") || text.startsWith("\\\\") || /^[A-Za-z]:[\\/]/.test(text);
  return absolute ? { kind: "path", path: text } : { kind: "refused", why: "skill_mount_relative" };
}


export function alreadyMounted(shelves: readonly string[], path: string): boolean {
  return shelves.some((each) => each.trim() === path.trim());
}


export function withLibrary(shelves: readonly string[], path: string): readonly string[] {
  return alreadyMounted(shelves, path) ? shelves : [...shelves, path.trim()];
}


export function withoutLibrary(shelves: readonly string[], path: string): readonly string[] {
  return shelves.filter((each) => each !== path);
}
