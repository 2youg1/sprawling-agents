// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A stored object's bytes as they arrive: one `Query::Bytes` window after
// another (`crates/wire/Spec.lean` §8-80, wire D14), joined where the
// last one ended, until they meet the object's end (client/Spec.lean §4-61).
// A PDF, a DOCX and a screenshot are read this way; what the bytes are is
// the view's to judge.

import type { B3Hash, BytesAnswer, Locator, Span } from "../wire";

// The most bytes the page holds to draw one file: the whole object stays
// in memory while a tool draws it.
export const DRAWN_BYTES_MAX = 64 * 1024 * 1024;

export interface Fetching {
  readonly version: B3Hash;
  // The object's length, once the first answer has said it.
  readonly size: number | null;
  readonly through: number;
  readonly parts: readonly Uint8Array[];
}

export type Fetched =
  | { readonly kind: "fetching"; readonly through: number; readonly size: number | null }
  | { readonly kind: "whole"; readonly bytes: Uint8Array }
  | { readonly kind: "too_large"; readonly size: number };

export function fetching(version: B3Hash): Fetching {
  return { version, size: null, through: 0, parts: [] };
}

export function nextBytes(at: Fetching): Span | null {
  void at;
  return null;
}

export function joinedBytes(at: Fetching, answer: BytesAnswer): Fetching {
  void answer;
  return at;
}

export function fetchedOf(at: Fetching): Fetched {
  return { kind: "fetching", through: at.through, size: at.size };
}

export function storedObject(locator: Locator): B3Hash | null {
  void locator;
  return null;
}
