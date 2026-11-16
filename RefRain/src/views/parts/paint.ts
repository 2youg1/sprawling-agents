// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { highlightCode, tagHighlighter, tags } from "@lezer/highlight";

import type { Ink, Piece } from "./code";

type Tree = Parameters<typeof highlightCode>[1];

interface Grammar {
  parse(text: string): Tree;
}

type Family =
  | "cpp"
  | "css"
  | "go"
  | "java"
  | "javascript"
  | "json"
  | "jsx"
  | "python"
  | "rust"
  | "tsx"
  | "typescript"
  | "yaml";

const LOAD: Record<Family, () => Promise<Grammar>> = {
  cpp: () => import("@lezer/cpp").then((grammar) => grammar.parser),
  css: () => import("@lezer/css").then((grammar) => grammar.parser),
  go: () => import("@lezer/go").then((grammar) => grammar.parser),
  java: () => import("@lezer/java").then((grammar) => grammar.parser),
  javascript: () => import("@lezer/javascript").then((grammar) => grammar.parser),
  json: () => import("@lezer/json").then((grammar) => grammar.parser),
  jsx: () =>
    import("@lezer/javascript").then((grammar) => grammar.parser.configure({ dialect: "jsx" })),
  python: () => import("@lezer/python").then((grammar) => grammar.parser),
  rust: () => import("@lezer/rust").then((grammar) => grammar.parser),
  tsx: () =>
    import("@lezer/javascript").then((grammar) => grammar.parser.configure({ dialect: "ts jsx" })),
  typescript: () =>
    import("@lezer/javascript").then((grammar) => grammar.parser.configure({ dialect: "ts" })),
  yaml: () => import("@lezer/yaml").then((grammar) => grammar.parser),
};

const FAMILY = new Map<string, Family>([
  ["c", "cpp"],
  ["cc", "cpp"],
  ["cpp", "cpp"],
  ["h", "cpp"],
  ["hpp", "cpp"],
  ["css", "css"],
  ["go", "go"],
  ["java", "java"],
  ["cjs", "javascript"],
  ["javascript", "javascript"],
  ["js", "javascript"],
  ["mjs", "javascript"],
  ["json", "json"],
  ["jsx", "jsx"],
  ["py", "python"],
  ["python", "python"],
  ["rs", "rust"],
  ["rust", "rust"],
  ["tsx", "tsx"],
  ["ts", "typescript"],
  ["typescript", "typescript"],
  ["yaml", "yaml"],
  ["yml", "yaml"],
]);

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

const loaded = new Map<Family, Promise<Grammar>>();

function grammarOf(family: Family): Promise<Grammar> {
  const known = loaded.get(family);
  if (known !== undefined) return known;
  const asked = LOAD[family]();
  loaded.set(family, asked);
  return asked;
}

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
      const ink = classes
        .split(" ")
        .map((name) => INK.get(name))
        .find((found) => found !== undefined);
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
