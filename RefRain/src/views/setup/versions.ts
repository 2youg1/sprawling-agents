// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key } from "../../core/lang";
import type { DoctorNewest, DoctorUnread } from "../../wire";

function parts(version: string): readonly number[] | null {
  const dotted = /\d+(?:\.\d+)+/u.exec(version)?.[0];
  return dotted === undefined ? null : dotted.split(".").map(Number);
}

export function behind(installed: string | null, newest: string | null): boolean {
  if (installed === null || newest === null) return false;
  const have = parts(installed);
  const want = parts(newest);
  if (have === null || want === null) return false;
  for (let at = 0; at < Math.max(have.length, want.length); at += 1) {
    const mine = have[at] ?? 0;
    const theirs = want[at] ?? 0;
    if (mine !== theirs) return mine < theirs;
  }
  return false;
}

const UNREAD: Record<DoctorUnread, Key> = {
  with_toolchain: "machine_unread_with_toolchain",
  many_brands: "machine_unread_many_brands",
  matches_browser: "machine_unread_matches_browser",
  this_project: "machine_unread_this_project",
  no_source: "machine_unread_no_source",
  unknown_item: "machine_unread_unknown_item",
};

export type Newest =
  { readonly version: string } | { readonly key: Key; readonly said: string | null };

export function newestOf(answer: DoctorNewest | undefined): Newest {
  if (answer === undefined || answer === "asking")
    return { key: "machine_version_asking", said: null };
  if ("read" in answer) return { version: answer.read.version };
  if ("unread" in answer) return { key: UNREAD[answer.unread.why], said: null };
  return { key: "machine_upstream_refused", said: answer.refused.said };
}

export function stillAsking(newest: Readonly<Record<string, DoctorNewest>>): readonly string[] {
  return Object.entries(newest)
    .filter(([, answer]) => answer === "asking")
    .map(([item]) => item);
}
