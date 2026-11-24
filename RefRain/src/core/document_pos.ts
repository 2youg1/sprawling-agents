// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { B3Hash, Encoding, TextEdit } from "../wire";

export interface EditorChange {
  readonly from: number;
  readonly to: number;
  readonly insert: string;
}

export interface Place {
  readonly line: number;
  readonly column: number;
}

export interface Positions<Version extends string = B3Hash> {
  readonly version: Version;
  readonly encoding: Encoding;

  readonly editor: string;

  readonly lineBreak: string;

  readonly bytes: (at: number) => number;

  readonly editorAt: (byte: number) => number;
  readonly place: (at: number) => Place;
}

const STRIDE = 1024;

const MARK = 0xfeff;

const ASTRAL = /[\u{10000}-\u{10FFFF}]/gu;
const CR = 0x0d;
const LF = 0x0a;

interface Checkpoint {
  readonly position: number;
  readonly offset: number;
  readonly byte: number;
  readonly line: number;
}

interface Step {
  readonly units: number;
  readonly editor: number;
  readonly bytes: number;
}

export function positionsOf<Version extends string>(
  version: Version,
  encoding: Encoding,
  text: string,
): Positions<Version> {
  const marked = text.codePointAt(0) === MARK && encoding !== "utf8";
  const wide = encoding === "utf16_le" || encoding === "utf16_be";
  const width = (code: number): number => (wide ? (code > 0xffff ? 4 : 2) : utf8Width(code));
  const step = (offset: number): Step => {
    const code = text.codePointAt(offset) ?? 0;
    if (code === CR && text.charCodeAt(offset + 1) === LF)
      return { units: 2, editor: 1, bytes: width(CR) * 2 };
    const units = code > 0xffff ? 2 : 1;
    return { units, editor: units, bytes: width(code) };
  };
  const first: Checkpoint = {
    position: 0,
    offset: marked ? 1 : 0,
    byte: marked ? width(MARK) : 0,
    line: 1,
  };
  const walk = checkpoints(text, first, step);
  const editor = text.slice(first.offset).replace(/\r\n?/gu, "\n");

  const walked = (
    from: Checkpoint,
    passes: (at: Checkpoint, next: Step) => boolean,
  ): Checkpoint => {
    let at = from;
    for (
      let next = step(at.offset);
      at.offset < text.length && !passes(at, next);
      next = step(at.offset)
    ) {
      at = {
        position: at.position + next.editor,
        offset: at.offset + next.units,
        byte: at.byte + next.bytes,
        line: at.line,
      };
    }
    return at;
  };
  const bytes = (target: number): number =>
    walked(
      nearest(walk, (each) => each.position <= target),
      (at, next) => at.position + next.editor > target,
    ).byte;
  const editorAt = (byte: number): number =>
    walked(
      nearest(walk, (each) => each.byte <= byte),
      (at, next) => at.byte + next.bytes > byte,
    ).position;
  const place = (target: number): Place => {
    const at = Math.min(Math.max(target, 0), editor.length);
    const from = nearest(walk, (each) => each.position <= at);
    let line = from.line;
    for (
      let index = editor.indexOf("\n", from.position);
      index >= 0 && index < at;
      index = editor.indexOf("\n", index + 1)
    ) {
      line += 1;
    }
    const head = editor.slice(editor.lastIndexOf("\n", at - 1) + 1, at);
    return { line, column: head.length - (head.match(ASTRAL)?.length ?? 0) + 1 };
  };
  return {
    version,
    encoding,
    editor,
    lineBreak: /\r\n?|\n/u.exec(text)?.[0] ?? "\n",
    bytes,
    editorAt,
    place,
  };
}

export function textEdits(
  positions: Pick<Positions, "bytes" | "lineBreak">,
  changes: readonly EditorChange[],
): TextEdit[] {
  return changes.map((change) => ({
    span: { start: positions.bytes(change.from), end: positions.bytes(change.to) },
    text:
      positions.lineBreak === "\n"
        ? change.insert
        : change.insert.replaceAll("\n", positions.lineBreak),
  }));
}

export function utf16At(text: string, byte: number): number {
  let at = 0;
  let walked = 0;
  for (const character of text) {
    walked += utf8Width(character.codePointAt(0) ?? 0);
    if (walked > byte) return at;
    at += character.length;
  }
  return at;
}

function checkpoints(
  text: string,
  first: Checkpoint,
  step: (offset: number) => Step,
): readonly Checkpoint[] {
  const kept = [first];
  let at = first;
  while (at.offset < text.length) {
    const next = step(at.offset);
    const line =
      at.line + (text.charCodeAt(at.offset) === CR || text.charCodeAt(at.offset) === LF ? 1 : 0);
    const moved = {
      position: at.position + next.editor,
      offset: at.offset + next.units,
      byte: at.byte + next.bytes,
      line,
    };
    if (Math.floor(moved.position / STRIDE) > Math.floor(at.position / STRIDE)) kept.push(moved);
    at = moved;
  }
  return kept;
}

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
