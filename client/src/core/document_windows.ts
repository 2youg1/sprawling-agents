// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The whole text of one document version, gathered from the windows the
// city answers (client-SPEC 4-46). `Query::Document` answers the first
// window; a version longer than one is in the city's content store and
// the rest is read by version through `Query::Range`, each window
// starting where the last one ended (`crates/wire/Spec.lean` §8-69,
// §8-70). The windows are bytes of the version, so the text is complete
// exactly when the windows reach the version's length.

import type { B3Hash, DocumentAnswer, Encoding, Format, RangeAnswer, Span } from "../wire";

// The longest version RefRain gathers whole and lets a person edit:
// sixty-four windows. A longer version shows its first window, read
// only, and says so, rather than saving a text it never held whole.
export const EDITABLE_BYTES_MAX = 4 * 1024 * 1024;

// A text version and how much of it has arrived: `through` is the byte
// the text reaches, and the text is whole when it reaches `bytes`, which
// is `null` for a version known only by its digest until its end is read.
export interface Gathering {
  readonly version: B3Hash;
  readonly format: Format;
  readonly encoding: Encoding;
  readonly bytes: number | null;
  readonly text: string;
  readonly through: number;
}

// What a document answer opens: a file that is not there, one the city
// cannot read (in the system's own words), bytes that are not text, or
// a text being gathered. An empty file is a text that is already whole,
// and can be written a first version.
export type Opened =
  | { readonly kind: "missing" }
  | { readonly kind: "unreadable"; readonly reason: string }
  | { readonly kind: "opaque"; readonly version: B3Hash; readonly bytes: number }
  | { readonly kind: "text"; readonly gathering: Gathering };

export function opened(answer: DocumentAnswer): Opened {
  const state = answer.state;
  if (state === "missing") return { kind: "missing" };
  if ("unreadable" in state) return { kind: "unreadable", reason: state.unreadable.reason };
  if ("empty" in state) {
    const empty = state.empty;
    return {
      kind: "text",
      gathering: { version: empty.version, format: empty.format, encoding: "utf8", bytes: 0, text: "", through: 0 },
    };
  }
  const held = state.held;
  if (held.body === "opaque") return { kind: "opaque", version: held.version, bytes: held.bytes };
  const body = held.body.text;
  return {
    kind: "text",
    gathering: {
      version: held.version,
      format: held.format,
      encoding: body.encoding,
      bytes: held.bytes,
      text: body.head.text,
      through: body.head.span.end,
    },
  };
}

// A version the page knows only by its digest: one that is not the
// file's current version. Its length is learnt from the empty window the
// city answers past its end, and it is read as UTF-8 because only a
// document answer states an encoding; such a version is read only, and
// a marked UTF-8 version's mark is three bytes either way.
export function recorded(version: B3Hash, format: Format): Gathering {
  return { version, format, encoding: "utf8", bytes: null, text: "", through: 0 };
}

export function whole(gathering: Gathering): boolean {
  return gathering.bytes !== null && gathering.through >= gathering.bytes;
}

// The bytes to ask for next, or `null` when the text is whole or the
// version is past what RefRain gathers. The city cuts the answer to one
// window, so the request names the whole rest.
export function nextSpan(gathering: Gathering): Span | null {
  const end = gathering.bytes ?? EDITABLE_BYTES_MAX;
  return whole(gathering) || end > EDITABLE_BYTES_MAX || gathering.through >= EDITABLE_BYTES_MAX
    ? null
    : { start: gathering.through, end };
}

// The text with one more window: only a window of this version that
// starts where the text ends, so a window answered twice, out of order
// or for another version leaves the text as it was. An empty window is
// the city saying the version ends there.
export function joined(gathering: Gathering, answer: RangeAnswer): Gathering {
  const window = answer.window;
  if (answer.version !== gathering.version || window.span.start !== gathering.through) return gathering;
  return window.span.end === window.span.start
    ? { ...gathering, bytes: gathering.through }
    : { ...gathering, text: gathering.text + window.text, through: window.span.end };
}

