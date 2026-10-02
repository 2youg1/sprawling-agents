// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { Answered } from "./answered";
import { UNLAID, answered, question } from "./replying";
import type { Laying } from "./replying";
import type { Block, Laid } from "../wire";

// Where the closure point falls is the city's rule and its tests
// (`crates/documents/src/layout/tests.rs`); these pin the page's half:
// which stretch goes out, and how an answer joins what is drawn.

function bytes(text: string): number {
  return new TextEncoder().encode(text).length;
}

function said(words: string, start: number): Block {
  return { paragraph: { span: { start, end: start + bytes(words) }, inline: [{ text: words }] } };
}

// The city's answer to a question: these blocks, read up to the end of
// `prefix`.
function laid(prefix: string, blocks: readonly Block[]): Answered<Laid> {
  return { kind: "held", value: { span: { start: 0, end: bytes(prefix) }, blocks } };
}

describe("a reply still being said", () => {
  test("carries only complete lines, and asks nothing until one arrives", () => {
    expect([
      question("", UNLAID, "streaming"),
      question("still saying", UNLAID, "streaming"),
      question("one\n\ntwo, still", UNLAID, "streaming"),
    ]).toEqual([null, null, "one\n\n"]);
  });

  test("appends each answer and asks next from where the blocks end", () => {
    const first = answered(UNLAID, "one\n\n", laid("one\n\n", [said("one", 0)]), "streaming");
    expect(first).toEqual({ blocks: [said("one", 0)], reached: 5, judged: "", refused: false });
    expect(question("one\n\ntwo, still", first, "streaming")).toBeNull();
    expect(question("one\n\ntwo, still\nmore\n\n", first, "streaming")).toBe("two, still\nmore\n\n");
    const second = answered(first, "two, still\nmore\n\n", laid("two, still\nmore\n\n", [said("two, still more", 0)]), "streaming");
    expect(second).toEqual({
      blocks: [said("one", 0), said("two, still more", 0)],
      reached: 22,
      judged: "",
      refused: false,
    });
  });

  test("does not ask again about lines the city read and left open", () => {
    const open = answered(UNLAID, "- a\n", laid("", []), "streaming");
    expect(open).toEqual({ blocks: [], reached: 0, judged: "- a\n", refused: false });
    expect([question("- a\n- b", open, "streaming"), question("- a\n- b\n", open, "streaming")]).toEqual([
      null,
      "- a\n- b\n",
    ]);
  });

  test("counts the city's bytes as UTF-8 and the text's length in code units", () => {
    const asked = "第一段 😀\n\n";
    const after = answered(UNLAID, asked, laid(asked, [said("第一段 😀", 0)]), "streaming");
    expect({ reached: after.reached, rest: `${asked}尾巴`.slice(after.reached) }).toEqual({ reached: 8, rest: "尾巴" });
  });
});

describe("a settled reply", () => {
  test("is asked whole, then from where a full window stopped, then not at all", () => {
    const text = "one\n\ntwo\n";
    expect(question(text, UNLAID, "settled")).toBe(text);
    const window = answered(UNLAID, text, laid("one\n\n", [said("one", 0)]), "settled");
    expect(question(text, window, "settled")).toBe("two\n");
    const whole = answered(window, "two\n", laid("two\n", [said("two", 0)]), "settled");
    expect({ whole, next: question(text, whole, "settled") }).toEqual({
      whole: { blocks: [said("one", 0), said("two", 0)], reached: 9, judged: null, refused: false },
      next: null,
    });
  });

  test("an answer that read nothing is not asked again", () => {
    const stuck = answered(UNLAID, "text", laid("", []), "settled");
    expect(question("text", stuck, "settled")).toBeNull();
  });
});

describe("either state", () => {
  test("an answer still on its way changes nothing", () => {
    const held: Laying = { blocks: [said("one", 0)], reached: 5, judged: null, refused: false };
    expect(answered(held, "two\n\n", { kind: "asking" }, "streaming")).toEqual(held);
  });

  test("a refused stretch stays raw and is never asked again", () => {
    const refused = answered(UNLAID, "a\u0000b\n\n", { kind: "unavailable", query: "Reply" }, "streaming");
    expect({
      refused,
      streaming: question("a\u0000b\n\nmore\n\n", refused, "streaming"),
      settled: question("a\u0000b\n\nmore", refused, "settled"),
    }).toEqual({ refused: { ...UNLAID, refused: true }, streaming: null, settled: null });
  });
});
