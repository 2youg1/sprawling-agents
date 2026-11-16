// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";
import { writable } from "svelte/store";
import type { Readable } from "svelte/store";

import { QUERIES } from "./asking";
import { putPreferences } from "./commands";
import type { Connection } from "./socket";
import { Address, Tag } from "../wire";
import type { Seq, SessionTags } from "../wire";

export const PIN: Tag = Tag.make("pin");

export interface Named {
  readonly city: Address;
  readonly room: Address;
  readonly began: Seq;

  readonly usual?: Tag | null;
}

const readOne = Schema.decodeOption(Tag);
const readCity = Schema.decodeOption(Address);

export function namedIn(
  city: string | null,
  room: Address,
  began: Seq,
  workspace: string | null,
): Named | null {
  const usual = workspace === null ? null : readTag(workspace);
  return city === null
    ? null
    : Option.getOrNull(
        Option.map(readCity(city), (named) => ({ city: named, room, began, usual })),
      );
}

export function readTag(raw: string): Tag | null {
  return Option.getOrNull(readOne(raw.trim().toLowerCase()));
}

function same(entry: Named, session: Named): boolean {
  return (
    entry.city === session.city && entry.room === session.room && entry.began === session.began
  );
}

export function tagsOf(held: readonly SessionTags[], session: Named): readonly Tag[] {
  const usual = session.usual ?? null;
  return held.find((entry) => same(entry, session))?.tags ?? (usual === null ? [] : [usual]);
}

export function kept(held: readonly SessionTags[], session: Named): boolean {
  return held.some((entry) => same(entry, session));
}

export function given(held: readonly SessionTags[], session: Named, tag: Tag): SessionTags {
  const tags = tagsOf(held, session);
  return named(session, tags.includes(tag) ? tags : [...tags, tag].sort());
}

export function stripped(held: readonly SessionTags[], session: Named, tag: Tag): SessionTags {
  return named(
    session,
    tagsOf(held, session).filter((each) => each !== tag),
  );
}

function named(session: Named, tags: readonly Tag[]): SessionTags {
  return { city: session.city, room: session.room, began: session.began, tags };
}

export function retagged(held: readonly SessionTags[], next: SessionTags): readonly SessionTags[] {
  const others = held.filter((entry) => !same(entry, next));
  return next.tags.length === 0 ? others : [...others, next];
}

export function inUse(sets: readonly (readonly Tag[])[]): readonly Tag[] {
  return [...new Set(sets.flat())].filter((tag) => tag !== PIN).sort();
}

export interface Tagging {
  readonly held: Readable<readonly SessionTags[]>;

  readonly retag: (next: SessionTags) => boolean;
}

export function keepTags(conn: Pick<Connection, "asking" | "command">): Tagging {
  const held = writable<readonly SessionTags[]>([]);
  conn.asking.ask(QUERIES.preferences).subscribe((answer) => {
    if (answer !== undefined && "preferences" in answer) held.set(answer.preferences.tags ?? []);
  });
  return {
    held,
    retag: (next) => {
      if (!conn.command(putPreferences({ tags: next }))) return false;
      held.update((now) => retagged(now, next));
      return true;
    },
  };
}
