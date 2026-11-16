// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { B3Hash, DocumentAnswer, Encoding, Format, RangeAnswer, Span } from "../wire";

export const EDITABLE_BYTES_MAX = 4 * 1024 * 1024;

export interface Gathering {
  readonly version: B3Hash;
  readonly format: Format;
  readonly encoding: Encoding;
  readonly bytes: number | null;
  readonly text: string;
  readonly through: number;
}

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
      gathering: {
        version: empty.version,
        format: empty.format,
        encoding: "utf8",
        bytes: 0,
        text: "",
        through: 0,
      },
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

export function recorded(version: B3Hash, format: Format): Gathering {
  return { version, format, encoding: "utf8", bytes: null, text: "", through: 0 };
}

export function whole(gathering: Gathering): boolean {
  return gathering.bytes !== null && gathering.through >= gathering.bytes;
}

export function nextSpan(gathering: Gathering): Span | null {
  const end = gathering.bytes ?? EDITABLE_BYTES_MAX;
  return whole(gathering) || end > EDITABLE_BYTES_MAX || gathering.through >= EDITABLE_BYTES_MAX
    ? null
    : { start: gathering.through, end };
}

export function joined(gathering: Gathering, answer: RangeAnswer): Gathering {
  const window = answer.window;
  if (answer.version !== gathering.version || window.span.start !== gathering.through)
    return gathering;
  return window.span.end === window.span.start
    ? { ...gathering, bytes: gathering.through }
    : { ...gathering, text: gathering.text + window.text, through: window.span.end };
}
