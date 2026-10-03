// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The tags a person gives sessions, and the one reserved tag that pins.
//
// **The tags are the person's, kept by the city** in the person's own
// `config.toml` beside the chords they rebound (`crates/wire/Spec.lean`
// §8-84), so a phone reaching the same city reads the same tags. This
// page draws its own copy at once and sends the change; every answer the
// city gives replaces the copy whole, as every other preference does.
//
// **A session is named the way the ledger files it**: the city, the room,
// and the line its stretch began at (`SessionLine::began`).
//
// **A session the person never tagged carries its workspace as a tag**
// (`crates/wire/spec/Answer/Sessions.lean` D27): the default is derived
// here and stored nowhere, so the first change the person makes to the
// set writes it down with the rest, and from then on the stored set is
// the whole truth.

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
  // The tag the session carries while the city keeps none for it: its
  // workspace, when that name reads as a tag.
  readonly usual?: Tag | null;
}

const readOne = Schema.decodeOption(Tag);
const readCity = Schema.decodeOption(Address);

// A session of the city the handshake named, as its tags name it; `null`
// when the city stated no name, which leaves nowhere to keep a tag.
export function namedIn(city: string | null, room: Address, began: Seq, workspace: string | null): Named | null {
  const usual = workspace === null ? null : readTag(workspace);
  return city === null ? null : Option.getOrNull(Option.map(readCity(city), (named) => ({ city: named, room, began, usual })));
}

// What a person typed, as the tag it means. Folded to lower case here and
// nowhere else, so `Bug` typed twice is one tag; the grammar is the
// city's and arrives through the generated schema.
export function readTag(raw: string): Tag | null {
  return Option.getOrNull(readOne(raw.trim().toLowerCase()));
}

function same(entry: Named, session: Named): boolean {
  return entry.city === session.city && entry.room === session.room && entry.began === session.began;
}

export function tagsOf(held: readonly SessionTags[], session: Named): readonly Tag[] {
  const usual = session.usual ?? null;
  return held.find((entry) => same(entry, session))?.tags ?? (usual === null ? [] : [usual]);
}

// Whether the city keeps a set for the session. A tag drawn from the
// default alone cannot be taken off - an empty set removes the entry,
// and the default comes back - so the menu offers no removal then.
export function kept(held: readonly SessionTags[], session: Named): boolean {
  return held.some((entry) => same(entry, session));
}

// The session's whole new set with `tag` added: the value one
// `PreferencePatch::Tags` carries.
export function given(held: readonly SessionTags[], session: Named, tag: Tag): SessionTags {
  const tags = tagsOf(held, session);
  return named(session, tags.includes(tag) ? tags : [...tags, tag].sort());
}

// The session's whole new set with `tag` taken off; empty removes it.
export function stripped(held: readonly SessionTags[], session: Named, tag: Tag): SessionTags {
  return named(session, tagsOf(held, session).filter((each) => each !== tag));
}

function named(session: Named, tags: readonly Tag[]): SessionTags {
  return { city: session.city, room: session.room, began: session.began, tags };
}

// One session's set landed on this page's copy, in the patch's meaning
// (wire §8-84): it replaces the session's set, and an empty set removes
// the entry. The order of the list is the city's to keep; nothing here
// reads it.
export function retagged(held: readonly SessionTags[], next: SessionTags): readonly SessionTags[] {
  const others = held.filter((entry) => !same(entry, next));
  return next.tags.length === 0 ? others : [...others, next];
}

// Every tag the listed sessions carry but the pin, once each and in
// lexical order: the row a person filters the sessions pane by. Read off
// the rows rather than the stored sets, so a workspace's default tag
// filters as well as a given one.
export function inUse(sets: readonly (readonly Tag[])[]): readonly Tag[] {
  return [...new Set(sets.flat())].filter((tag) => tag !== PIN).sort();
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
      held.update((now) => retagged(now, next));
      return true;
    },
  };
}
