// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const TOKEN = 4;
const ROUNDS = 7;

const SECTION = [
  "Reading the city first, then the plan it asks for.",
  "",
  "- `Sprawling/kernel` holds the **Ledger** and every rule on it",
  "- `Sprawling/runtime` drives a turn",
  "- `Sprawling/app` wires the two together",
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

function drawBlock(source: string): HTMLElement {
  const lines = source.split("\n");
  if (source.startsWith("- ")) {
    const list = document.createElement("ul");
    for (const line of lines) list.append(element("li", line.slice(2)));
    return list;
  }
  if (source.startsWith("|")) {
    const table = document.createElement("table");
    for (const line of lines.filter((each) => !each.startsWith("|---"))) {
      const row = document.createElement("tr");
      for (const cell of line.split("|").slice(1, -1)) row.append(element("td", cell.trim()));
      table.append(row);
    }
    return table;
  }
  if (source.startsWith("```")) return element("pre", lines.slice(1, -1).join("\n"));
  return element("p", source.replace(/^#+ /u, ""));
}

function blocksBefore(text: string, end: number): string[] {
  return text
    .slice(0, end)
    .split("\n\n")
    .filter((each) => each.trim() !== "");
}

function closedIn(text: string): number {
  const blank = text.lastIndexOf("\n\n");
  return blank < 0 ? 0 : blank + 2;
}

function draw(host: HTMLElement, text: string): void {
  host.replaceChildren(...blocksBefore(text, text.length).map(drawBlock));
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
      const closed = closedIn(text);
      if (closed !== laidLength) {
        laid.append(...blocksBefore(text, closed).slice(laid.childElementCount).map(drawBlock));
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
  return quantile(
    [...values].sort((a, b) => a - b),
    0.5,
  );
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
