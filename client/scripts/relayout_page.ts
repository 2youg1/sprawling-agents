// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The page half of the relayout probe (`relayout.ts` builds and runs
// it). A reply is fed in four-character tokens into two shapes of a
// live turn, and after each token the probe forces layout and reads
// the clock:
//   raw   - the whole text in one pre-wrapped node, laid out as blocks
//           only when the record takes over;
//   block - the blocks `closedUpTo` has closed drawn as blocks, the open
//           tail as text (what `talk/saying.svelte` draws).
// Both shapes build their blocks with `blocks` and the element each
// `prose.svelte` branch uses, so the laid-out part costs what the real
// page costs, less the styling the stylesheet adds to both alike. The two run interleaved, so a slow moment lands on both.

import { blocks, closedUpTo } from "../src/core/prose";
import type { Block } from "../src/core/prose";

const TOKEN = 4;
const ROUNDS = 7;

const SECTION = [
  "Reading the city first, then the plan it asks for.",
  "",
  "- `crates/kernel` holds the **Ledger** and every rule on it",
  "- `crates/runtime` drives a turn",
  "- `crates/sprawling` wires the two together",
  "",
  "| step | what | where |",
  "|---|---|---|",
  "| 1 | read | kernel |",
  "| 2 | plan | runtime |",
  "| 3 | write | sprawling |",
  "",
  "```rust",
  "fn main() {",
  '    println!("one binary");',
  "}",
  "```",
  "",
  "## Next",
  "",
  "The plan names each step with the file it touches, so a reader can check it against the diff.",
  "",
].join("\n");

const REPLY = Array.from({ length: 6 }, () => SECTION).join("\n");

function element(tag: string, text: string): HTMLElement {
  const node = document.createElement(tag);
  node.textContent = text;
  return node;
}

function words(block: Block): string {
  switch (block.kind) {
    case "heading":
    case "paragraph":
    case "quote":
      return block.inline.map((part) => part.text).join("");
    case "code":
      return block.text;
    case "list":
      return "";
    case "table":
      return "";
  }
}

function drawBlock(block: Block): HTMLElement {
  switch (block.kind) {
    case "list": {
      const list = document.createElement(block.ordered ? "ol" : "ul");
      for (const item of block.items) list.append(element("li", item.map((part) => part.text).join("")));
      return list;
    }
    case "table": {
      const table = document.createElement("table");
      for (const row of block.rows) {
        const line = document.createElement("tr");
        for (const cell of row) line.append(element("td", cell));
        table.append(line);
      }
      return table;
    }
    case "code":
      return element("pre", words(block));
    case "quote":
      return element("blockquote", words(block));
    case "heading":
    case "paragraph":
      return element("p", words(block));
  }
}

function draw(host: HTMLElement, text: string): void {
  host.replaceChildren(...blocks(text).map(drawBlock));
}

interface Turn {
  readonly root: HTMLElement;
  feed(text: string): void;
}

function rawTurn(root: HTMLElement): Turn {
  const tail = document.createElement("div");
  tail.style.whiteSpace = "pre-wrap";
  const words = document.createTextNode("");
  tail.append(words);
  root.append(tail);
  return {
    root,
    feed(text) {
      words.data = text;
    },
  };
}

function blockTurn(root: HTMLElement): Turn {
  const laid = document.createElement("div");
  const tail = document.createElement("div");
  tail.style.whiteSpace = "pre-wrap";
  const words = document.createTextNode("");
  tail.append(words);
  root.append(laid, tail);
  let laidLength = 0;
  return {
    root,
    feed(text) {
      const closed = closedUpTo(text);
      if (closed !== laidLength) {
        // `prose.svelte` re-reads every closed block, and its unkeyed
        // each leaves the DOM of a block it already drew untouched.
        laid.append(...blocks(text.slice(0, closed)).slice(laid.childElementCount).map(drawBlock));
        laidLength = closed;
      }
      words.data = text.slice(closed);
    },
  };
}

interface Reading {
  readonly perToken: number[];
  readonly takeover: number;
  readonly jump: number;
}

function stream(turn: Turn): Reading {
  const perToken: number[] = [];
  for (let end = TOKEN; end < REPLY.length + TOKEN; end += TOKEN) {
    const start = performance.now();
    turn.feed(REPLY.slice(0, end));
    void turn.root.getBoundingClientRect();
    perToken.push(performance.now() - start);
  }
  const before = turn.root.offsetHeight;
  const start = performance.now();
  draw(turn.root, REPLY);
  const after = turn.root.offsetHeight;
  return { perToken, takeover: performance.now() - start, jump: Math.abs(after - before) };
}

function stage(): HTMLElement {
  const root = document.createElement("div");
  root.style.width = "720px";
  root.style.lineHeight = "1.6";
  document.body.append(root);
  return root;
}

function quantile(sorted: readonly number[], q: number): number {
  return sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))] ?? 0;
}

function median(values: readonly number[]): number {
  return quantile([...values].sort((a, b) => a - b), 0.5);
}

function summary(shape: string, rounds: readonly Reading[]): string {
  const tokens = rounds.flatMap((round) => round.perToken).sort((a, b) => a - b);
  const mean = tokens.reduce((sum, value) => sum + value, 0) / tokens.length;
  const us = (ms: number) => `${(ms * 1000).toFixed(0)} us`;
  return [
    shape.padEnd(6),
    `mean ${us(mean)}`,
    `p50 ${us(quantile(tokens, 0.5))}`,
    `p99 ${us(quantile(tokens, 0.99))}`,
    `max ${us(quantile(tokens, 1))}`,
    `takeover ${median(rounds.map((round) => round.takeover)).toFixed(2)} ms`,
    `jump ${median(rounds.map((round) => round.jump)).toFixed(0)} px`,
  ].join("  ");
}

const raw: Reading[] = [];
const block: Reading[] = [];
for (let round = 0; round < ROUNDS; round += 1) {
  raw.push(stream(rawTurn(stage())));
  block.push(stream(blockTurn(stage())));
  document.body.replaceChildren();
}

const result = document.createElement("pre");
result.id = "relayout";
result.textContent = [
  `${String(Math.ceil(REPLY.length / TOKEN))} tokens, ${String(REPLY.length)} characters, ${String(ROUNDS)} rounds`,
  summary("raw", raw),
  summary("block", block),
].join("\n");
document.body.append(result);
