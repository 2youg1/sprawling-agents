// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The three coordinates of one version of a text document, and the one
// place they are converted (client-SPEC 4-46, refrain roadmap §4-8).
//
// **The wire counts bytes, the editor counts UTF-16 code units of a
// folded text.** A version's bytes are its identity
// (`crates/documents/Spec.lean` D2), so every span the city answers or
// accepts is a byte span. The editor holds the version's text with its
// byte-order mark dropped and every line break - `\r\n`, `\r`, `\n` -
// read as one `\n`, because that is how CodeMirror splits lines, and an
// editor that kept `\r` inside a line would let a person type between
// `\r` and `\n`. A person reads characters, so a column is counted in
// code points. All three are answered here, against the version they
// belong to, so a position measured on one version is never spent on
// another.

import type { B3Hash, Encoding, TextEdit } from "../wire";

// One change the editor made, in the editor's coordinates of the
// version it was made on: the stretch `[from, to)` replaced by `insert`,
// whose line breaks are the editor's `\n`.
export interface EditorChange {
  readonly from: number;
  readonly to: number;
  readonly insert: string;
}

// Where an editor position is, as a person counts: lines from 1, and
// characters (code points) from 1 within the line.
export interface Place {
  readonly line: number;
  readonly column: number;
}

export interface Positions {
  readonly version: B3Hash;
  readonly encoding: Encoding;
  // The text the editor holds: the mark dropped, every line break `\n`.
  readonly editor: string;
  // How this version writes a line break, which is how a new one is
  // written back: the first break it has, or `\n` when it has none.
  readonly lineBreak: string;
  // The byte of the version an editor position stands at. A position
  // inside a surrogate pair stands at the pair's first byte.
  readonly bytes: (at: number) => number;
  // The editor position a byte of the version falls in. A byte inside a
  // character - a multi-byte character, the two halves of `\r\n`, the
  // mark - falls at that character's start.
  readonly editorAt: (byte: number) => number;
  readonly place: (at: number) => Place;
}

// How many editor positions lie between two checkpoints: a conversion
// walks at most this far from the nearest one.
const STRIDE = 1024;

const MARK = 0xfeff;
// A character outside the basic plane: two code units, one character.
const ASTRAL = /[\u{10000}-\u{10FFFF}]/gu;
const CR = 0x0d;
const LF = 0x0a;

// A place in the walk: an editor position, the original text's offset
// and the byte it stands at, and how many line breaks lie before it.
// One is kept for every `STRIDE` editor positions, at the first
// character boundary at or past each stride line.
interface Checkpoint {
  readonly position: number;
  readonly offset: number;
  readonly byte: number;
  readonly line: number;
}

// How one character of the original text reads: the code units it
// takes there, the editor positions it takes, and its bytes.
interface Step {
  readonly units: number;
  readonly editor: number;
  readonly bytes: number;
}

export function positionsOf(version: B3Hash, encoding: Encoding, text: string): Positions {
  const marked = text.codePointAt(0) === MARK && encoding !== "utf8";
  const wide = encoding === "utf16_le" || encoding === "utf16_be";
  const width = (code: number): number => (wide ? (code > 0xffff ? 4 : 2) : utf8Width(code));
  const step = (offset: number): Step => {
    const code = text.codePointAt(offset) ?? 0;
    if (code === CR && text.charCodeAt(offset + 1) === LF) return { units: 2, editor: 1, bytes: width(CR) * 2 };
    const units = code > 0xffff ? 2 : 1;
    return { units, editor: units, bytes: width(code) };
  };
  const first: Checkpoint = { position: 0, offset: marked ? 1 : 0, byte: marked ? width(MARK) : 0, line: 1 };
  const walk = checkpoints(text, first, step);
  const editor = text.slice(first.offset).replace(/\r\n?/gu, "\n");
  // Walks on from a checkpoint until the next character would pass what
  // `passes` asks for. The walk stops on a character's start, which is
  // what puts a position or a byte inside a character at its start.
  const walked = (from: Checkpoint, passes: (at: Checkpoint, next: Step) => boolean): Checkpoint => {
    let at = from;
    for (let next = step(at.offset); at.offset < text.length && !passes(at, next); next = step(at.offset)) {
      at = { position: at.position + next.editor, offset: at.offset + next.units, byte: at.byte + next.bytes, line: at.line };
    }
    return at;
  };
  const bytes = (target: number): number =>
    walked(nearest(walk, (each) => each.position <= target), (at, next) => at.position + next.editor > target).byte;
  const editorAt = (byte: number): number =>
    walked(nearest(walk, (each) => each.byte <= byte), (at, next) => at.byte + next.bytes > byte).position;
  const place = (target: number): Place => {
    const at = Math.min(Math.max(target, 0), editor.length);
    const from = nearest(walk, (each) => each.position <= at);
    let line = from.line;
    for (let index = editor.indexOf("\n", from.position); index >= 0 && index < at; index = editor.indexOf("\n", index + 1)) {
      line += 1;
    }
    const head = editor.slice(editor.lastIndexOf("\n", at - 1) + 1, at);
    return { line, column: head.length - (head.match(ASTRAL)?.length ?? 0) + 1 };
  };
  return { version, encoding, editor, lineBreak: /\r\n?|\n/u.exec(text)?.[0] ?? "\n", bytes, editorAt, place };
}

// Each change written as the baseline's bytes and the text that takes
// their place, its line breaks written the way the version writes them.
// The changes are the editor's own, in document order and apart, so
// the edits are too (`crates/documents/Spec.lean` D8).
export function textEdits(positions: Positions, changes: readonly EditorChange[]): TextEdit[] {
  return changes.map((change) => ({
    span: { start: positions.bytes(change.from), end: positions.bytes(change.to) },
    text: positions.lineBreak === "\n" ? change.insert : change.insert.replaceAll("\n", positions.lineBreak),
  }));
}

// The UTF-16 offset in `text` of the character UTF-8 byte `byte` falls
// in: the coordinate a reply's answer is spent in. A reply has no
// version, so this is the same conversion without a `Positions`; a byte
// inside a character falls at that character's start, and a byte past
// the text at its end.
export function utf16At(text: string, byte: number): number {
  return text.length - text.length + byte - byte;
}

function checkpoints(text: string, first: Checkpoint, step: (offset: number) => Step): readonly Checkpoint[] {
  const kept = [first];
  let at = first;
  while (at.offset < text.length) {
    const next = step(at.offset);
    const line = at.line + (text.charCodeAt(at.offset) === CR || text.charCodeAt(at.offset) === LF ? 1 : 0);
    const moved = { position: at.position + next.editor, offset: at.offset + next.units, byte: at.byte + next.bytes, line };
    if (Math.floor(moved.position / STRIDE) > Math.floor(at.position / STRIDE)) kept.push(moved);
    at = moved;
  }
  return kept;
}

// The last checkpoint the test admits; the checkpoints climb in every
// coordinate, so the admitted ones are a prefix.
function nearest(walk: readonly Checkpoint[], admits: (each: Checkpoint) => boolean): Checkpoint {
  let low = 0;
  let high = walk.length;
  while (low < high) {
    const middle = (low + high) >> 1;
    const each = walk[middle];
    if (each !== undefined && admits(each)) low = middle + 1;
    else high = middle;
  }
  return walk[Math.max(0, low - 1)] ?? { position: 0, offset: 0, byte: 0, line: 1 };
}

function utf8Width(code: number): number {
  return code < 0x80 ? 1 : code < 0x800 ? 2 : code < 0x10000 ? 3 : 4;
}
