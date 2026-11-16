// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { face, folded } from "./keys";
import type { Pressed } from "./press";

export const LINE_MOVES = [
  "line.next",
  "line.previous",
  "line.first",
  "line.last",
  "line.open",
  "line.close",
] as const;

export type LineMove = (typeof LINE_MOVES)[number];

export const LINE_KEYS: Readonly<Record<LineMove, readonly (readonly string[])[]>> = {
  "line.next": [["ArrowDown"], ["j"]],
  "line.previous": [["ArrowUp"], ["k"]],
  "line.first": [["Home"], ["g", "g"]],
  "line.last": [["End"], ["G"]],
  "line.open": [["Enter"]],
  "line.close": [["Escape"]],
};

export const SEQUENCE_MS = 1000;

function sameKeys(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((key, at) => right[at] === key);
}

function moveOf(typed: readonly string[]): LineMove | undefined {
  return LINE_MOVES.find((move) => LINE_KEYS[move].some((keys) => sameKeys(keys, typed)));
}

function begins(typed: readonly string[]): boolean {
  return LINE_MOVES.some((move) =>
    LINE_KEYS[move].some(
      (keys) => keys.length > typed.length && sameKeys(keys.slice(0, typed.length), typed),
    ),
  );
}

export function lineWalker(): (pressed: Pressed, at: number) => LineMove | null {
  let held: readonly string[] = [];
  let heldAt = Number.NEGATIVE_INFINITY;
  return (pressed, at) => {
    const was = held;
    held = [];
    if (pressed.target === "field" || pressed.ctrlKey || pressed.metaKey || pressed.altKey)
      return null;
    const continued = at - heldAt <= SEQUENCE_MS ? [...was, pressed.key] : [pressed.key];
    heldAt = at;
    for (const typed of [continued, [pressed.key]]) {
      const move = moveOf(typed);
      if (move !== undefined) return move;
      if (begins(typed)) {
        held = typed;
        return null;
      }
    }
    return null;
  };
}

function lineFace(key: string): string {
  return key.length === 1 ? key : face(key);
}

export function lineFaces(move: LineMove): readonly string[] {
  return LINE_KEYS[move].map((keys) => keys.map(lineFace).join(""));
}

function typeable(letter: string): boolean {
  return /^[a-z0-9]$/.test(letter);
}

export function initialOf(name: string, slug: string): string {
  const first = folded(name.trim().slice(0, 1));
  return typeable(first) ? first : folded(slug.slice(0, 1));
}

export function initialTyped(pressed: Pressed): string | null {
  if (pressed.target === "field" || pressed.ctrlKey || pressed.metaKey || pressed.altKey)
    return null;
  const letter = folded(pressed.key);
  return typeable(letter) ? letter : null;
}
