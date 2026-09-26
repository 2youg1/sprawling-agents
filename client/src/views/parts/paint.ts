// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The highlighter itself, and the only file that knows a grammar exists.
// It is a lazy chunk: `code.ts` imports it on the first piece of code a
// screen shows, and each grammar below is a chunk of its own, so a page
// that only ever shows Rust downloads the Rust grammar and no other
// (client-SPEC 4-26).

import { highlightCode, tagHighlighter, tags } from "@lezer/highlight";

import type { Ink, Piece } from "./code";

type Tree = Parameters<typeof highlightCode>[1];

interface Grammar {
  parse(text: string): Tree;
}

// The grammars this build carries. The tsx and jsx variants share the
// JavaScript grammar with TypeScript and differ in the flags it takes.
type Family =
  | "cpp" | "css" | "go" | "java" | "javascript" | "json" | "jsx"
  | "python" | "rust" | "tsx" | "typescript" | "yaml";

const LOAD: Record<Family, () => Promise<Grammar>> = {
  cpp: () => import("@lezer/cpp").then((grammar) => grammar.parser),
  css: () => import("@lezer/css").then((grammar) => grammar.parser),
  go: () => import("@lezer/go").then((grammar) => grammar.parser),
  java: () => import("@lezer/java").then((grammar) => grammar.parser),
  javascript: () => import("@lezer/javascript").then((grammar) => grammar.parser),
  json: () => import("@lezer/json").then((grammar) => grammar.parser),
  jsx: () => import("@lezer/javascript").then((grammar) => grammar.parser.configure({ dialect: "jsx" })),
  python: () => import("@lezer/python").then((grammar) => grammar.parser),
  rust: () => import("@lezer/rust").then((grammar) => grammar.parser),
  tsx: () => import("@lezer/javascript").then((grammar) => grammar.parser.configure({ dialect: "ts jsx" })),
  typescript: () => import("@lezer/javascript").then((grammar) => grammar.parser.configure({ dialect: "ts" })),
  yaml: () => import("@lezer/yaml").then((grammar) => grammar.parser),
};

// Which grammar a name asks for. A file's extension and a Markdown
// fence's word are looked up in the same table, so `rs` and `rust` can
// never name two different grammars. A name the table does not hold is
// drawn in one ink rather than guessed at.
const FAMILY = new Map<string, Family>([
  ["c", "cpp"], ["cc", "cpp"], ["cpp", "cpp"], ["h", "cpp"], ["hpp", "cpp"],
  ["css", "css"],
  ["go", "go"],
  ["java", "java"],
  ["cjs", "javascript"], ["javascript", "javascript"], ["js", "javascript"], ["mjs", "javascript"],
  ["json", "json"],
  ["jsx", "jsx"],
  ["py", "python"], ["python", "python"],
  ["rs", "rust"], ["rust", "rust"],
  ["tsx", "tsx"],
  ["ts", "typescript"], ["typescript", "typescript"],
  ["yaml", "yaml"], ["yml", "yaml"],
]);

// The four inks the theme can hold apart at note size, each claiming
// every tag below its parent: `lineComment` is a comment, `character`
// and `regexp` are strings, `integer` is a number, `controlKeyword` is
// a word.
const HIGHLIGHT = tagHighlighter([
  { tag: tags.comment, class: "comment" },
  { tag: tags.string, class: "string" },
  { tag: tags.number, class: "number" },
  { tag: tags.keyword, class: "word" },
]);

const INK = new Map<string, Ink>([
  ["comment", "comment"],
  ["string", "string"],
  ["number", "number"],
  ["word", "word"],
]);

// A grammar is built once per page: the TypeScript variant is a new
// parser each time it is configured, and a screen repaints often.
const loaded = new Map<Family, Promise<Grammar>>();

function grammarOf(family: Family): Promise<Grammar> {
  const known = loaded.get(family);
  if (known !== undefined) return known;
  const asked = LOAD[family]();
  loaded.set(family, asked);
  return asked;
}

// The word that picks the grammar: what follows the last dot of the
// file's name, or the whole name when it has none, which is how a fence
// word such as `rust` arrives.
function nameOf(source: string): string {
  const name = source.slice(source.lastIndexOf("/") + 1);
  return name.slice(name.lastIndexOf(".") + 1).toLowerCase();
}

export async function paint(text: string, source: string): Promise<readonly Piece[]> {
  const family = FAMILY.get(nameOf(source));
  if (family === undefined) return [{ ink: "plain", text }];
  const grammar = await grammarOf(family);
  const pieces: Piece[] = [];
  let plain = "";
  highlightCode(
    text,
    grammar.parse(text),
    HIGHLIGHT,
    (run, classes) => {
      const ink = classes.split(" ").map((name) => INK.get(name)).find((found) => found !== undefined);
      if (ink === undefined) {
        plain += run;
        return;
      }
      if (plain !== "") pieces.push({ ink: "plain", text: plain });
      plain = "";
      pieces.push({ ink, text: run });
    },
    () => {
      plain += "\n";
    },
  );
  if (plain !== "") pieces.push({ ink: "plain", text: plain });
  return pieces;
}
