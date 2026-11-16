// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Result, Schema } from "effect";

import { B3Hash } from "../wire";
import type { BytesAnswer, Locator, Span } from "../wire";

export const DRAWN_BYTES_MAX = 64 * 1024 * 1024;

export interface Fetching {
  readonly version: B3Hash;

  readonly size: number | null;
  readonly through: number;
  readonly parts: readonly Uint8Array[];
}

export type Fetched =
  | { readonly kind: "fetching"; readonly through: number; readonly size: number | null }
  | { readonly kind: "whole"; readonly bytes: Uint8Array<ArrayBuffer> }
  | { readonly kind: "too_large"; readonly size: number };

export function fetching(version: B3Hash): Fetching {
  return { version, size: null, through: 0, parts: [] };
}

export function nextBytes(at: Fetching): Span | null {
  if (at.size === null) return { start: at.through, end: Number.MAX_SAFE_INTEGER };
  if (at.size > DRAWN_BYTES_MAX || at.through >= at.size) return null;
  return { start: at.through, end: at.size };
}

export function joinedBytes(at: Fetching, answer: BytesAnswer): Fetching {
  if (answer.version !== at.version || answer.span.start !== at.through) return at;
  const bytes = decoded(answer.base64);
  if (bytes?.length !== answer.span.end - answer.span.start) return at;
  return {
    version: at.version,
    size: answer.size,
    through: answer.span.end,
    parts: [...at.parts, bytes],
  };
}

export function fetchedOf(at: Fetching): Fetched {
  if (at.size !== null && at.size > DRAWN_BYTES_MAX) return { kind: "too_large", size: at.size };
  if (at.size === null || at.through < at.size)
    return { kind: "fetching", through: at.through, size: at.size };
  const whole = new Uint8Array(at.through);
  let offset = 0;
  for (const part of at.parts) {
    whole.set(part, offset);
    offset += part.length;
  }
  return { kind: "whole", bytes: whole };
}

const WHOLE_OBJECT = "cas:b3-";

export function storedObject(locator: Locator): B3Hash | null {
  if (!locator.startsWith(WHOLE_OBJECT)) return null;
  return Option.getOrNull(Schema.decodeUnknownOption(B3Hash)(locator.slice(WHOLE_OBJECT.length)));
}

function decoded(base64: string): Uint8Array | null {
  const binary = Result.getOrNull(Result.try(() => atob(base64)));
  return binary === null ? null : Uint8Array.from(binary, (character) => character.charCodeAt(0));
}
