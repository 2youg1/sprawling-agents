// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The tags a person gives sessions, and the one reserved tag that pins.
//
// **The tags are the person's, kept by the city** in the person's own
// `config.toml` beside the chords they rebound (`crates/wire/Spec.lean`
// §8-80), so a phone reaching the same city reads the same tags. This
// page draws its own copy at once and sends the change; every answer the
// city gives replaces the copy whole, as every other preference does.
//
// **A session is named the way the ledger files it**: the city, the room,
// and the line its stretch began at (`SessionLine::began`).

import { Option, Schema } from "effect";
import { writable } from "svelte/store";
import type { Readable } from "svelte/store";

import { QUERIES } from "./asking";
import { putPreferences } from "./commands";
import type { Connection } from "./socket";
import { Address, Tag } from "../wire";
import type { Seq, SessionTags } from "../wire";

// The tag that pins a session above the buildings. A word of this page
// rather than of the wire: on the wire `pin` is a tag like any other.
export const PIN: Tag = Tag.make("pin");

// A session as its tags name it.
export interface Named {
  readonly city: Address;
  readonly room: Address;
  readonly began: Seq;
}

const readOne = Schema.decodeOption(Tag);
const readCity = Schema.decodeOption(Address);

// A session of the city the handshake named, as its tags name it; `null`
// when the city stated no name, which leaves nowhere to keep a tag.
export function namedIn(city: string | null, room: Address, began: Seq): Named | null {
  return city === null ? null : Option.getOrNull(Option.map(readCity(city), (named) => ({ city: named, room, began })));
}

// What a person typed, as the tag it means. Folded to lower case here and
// nowhere else, so `Bug` typed twice is one tag; the grammar is the
// city's and arrives through the generated schema.
export function readTag(raw: string): Tag | null {
  return Option.getOrNull(readOne(raw));
}

function same(entry: Named, session: Named): boolean {
  return entry.city === session.city && entry.room === session.room && entry.began === session.began;
}

export function tagsOf(held: readonly SessionTags[], session: Named): readonly Tag[] {
  return held.find((entry) => same(entry, session))?.tags ?? [];
}

// The session's whole new set with `tag` added: the value one
// `PreferencePatch::Tags` carries.
export function given(held: readonly SessionTags[], session: Named, tag: Tag): SessionTags {
  const tags = tagsOf(held, session);
  return named(session, tags);
}

// The session's whole new set with `tag` taken off; empty removes it.
export function stripped(held: readonly SessionTags[], session: Named, tag: Tag): SessionTags {
  return named(session, tagsOf(held, session));
}

function named(session: Named, tags: readonly Tag[]): SessionTags {
  return { city: session.city, room: session.room, began: session.began, tags };
}

// One session's set landed on this page's copy, in the patch's meaning
// (wire §8-80): it replaces the session's set, and an empty set removes
// the entry. The order of the list is the city's to keep; nothing here
// reads it.
export function retagged(held: readonly SessionTags[], next: SessionTags): readonly SessionTags[] {
  const others = held.filter((entry) => !same(entry, next));
  return others;
}

// Every tag this city's sessions carry but the pin, once each and in
// lexical order: the row a person filters the sessions pane by.
export function inUse(held: readonly SessionTags[], city: string): readonly Tag[] {
  const all = held.filter((entry) => entry.city === city).flatMap((entry) => entry.tags);
  return all;
}

export interface Tagging {
  readonly held: Readable<readonly SessionTags[]>;
  // Lands one session's new set here and tells the city; false when the
  // link is down, which leaves this page's copy as it was.
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
      return true;
    },
  };
}
