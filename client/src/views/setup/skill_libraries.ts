// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the skills page does with the folder a person points at: the
// whole directory is mounted read-only as one more shelf, and the city's
// own `[skills] shelves` table is the only authority on which folders are
// mounted (client/Spec.lean §4-31). The page decides nothing about the
// shape of a path — the city reads it with the same rule as the
// configuration file — so this module only says what the box holds, and
// how one path joins or leaves the ordered table it was read from.

import type { Key } from "../../core/lang";

// What the folder box holds: nothing yet, a path to offer the city, or a
// shape the city could never read.
export type Offered =
  | { readonly kind: "blank" }
  | { readonly kind: "refused"; readonly why: Key }
  | { readonly kind: "path"; readonly path: string };

// A path is either under this person's home (`~/...`, which the city
// expands itself) or absolute. A relative path has no base on the city's
// machine, so the page says so before the frame is sent.
export function offered(written: string): Offered {
  const text = written.trim();
  if (text === "") return { kind: "blank" };
  if (text === "~" || text === "~/") return { kind: "refused", why: "skill_mount_home" };
  if (text.startsWith("~/") || text.startsWith("~\\")) return { kind: "path", path: text };
  const absolute = text.startsWith("/") || text.startsWith("\\\\") || /^[A-Za-z]:[\\/]/.test(text);
  return absolute ? { kind: "path", path: text } : { kind: "refused", why: "skill_mount_relative" };
}

// The folders mounted now, in the order the city answered them.
export function mounted(shelves: readonly string[]): readonly string[] {
  return [...shelves];
}

// Whether this exact spelling is already mounted, compared as the city
// compares it: the text it was given, trimmed.
export function alreadyMounted(shelves: readonly string[], path: string): boolean {
  return shelves.some((each) => each.trim() === path.trim());
}

// The whole table after adding one folder. A folder already on the list
// is not added twice: a shelf's index is one row per name, and a second
// identical row would only be a second way to say the same thing.
export function withLibrary(shelves: readonly string[], path: string): readonly string[] {
  return alreadyMounted(shelves, path) ? mounted(shelves) : [...shelves, path.trim()];
}

// The whole table after taking one folder away, by its exact spelling.
export function withoutLibrary(shelves: readonly string[], path: string): readonly string[] {
  return shelves.filter((each) => each !== path);
}
