// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a model said, read as blocks a page can draw. A small reading of
// Markdown - headings, lists, fenced code, paragraphs, and the three
// inline marks - produced as data rather than as HTML, so nothing a
// model writes is ever handed to `innerHTML`.

export type Inline =
  | { readonly kind: "text"; readonly text: string }
  | { readonly kind: "code"; readonly text: string }
  | { readonly kind: "strong"; readonly text: string }
  | { readonly kind: "em"; readonly text: string }
  | { readonly kind: "link"; readonly text: string; readonly href: string };

export type Block =
  | { readonly kind: "heading"; readonly level: number; readonly inline: readonly Inline[] }
  | { readonly kind: "paragraph"; readonly inline: readonly Inline[] }
  | { readonly kind: "list"; readonly ordered: boolean; readonly items: readonly (readonly Inline[])[] }
  | { readonly kind: "code"; readonly lang: string; readonly text: string }
  | { readonly kind: "quote"; readonly inline: readonly Inline[] }
  | { readonly kind: "table"; readonly rows: readonly (readonly string[])[] };

const INLINE = /(`[^`]+`)|(\*\*[^*]+\*\*)|(\*[^*]+\*)|(\[[^\]]+\]\((https?:\/\/[^)\s]+)\))/g;

export function inline(text: string): Inline[] {
  const out: Inline[] = [];
  let last = 0;
  for (const match of text.matchAll(INLINE)) {
    const at = match.index;
    if (at > last) {
      out.push({ kind: "text", text: text.slice(last, at) });
    }
    const whole = match[0];
    if (match[1] !== undefined) {
      out.push({ kind: "code", text: whole.slice(1, -1) });
    } else if (match[2] !== undefined) {
      out.push({ kind: "strong", text: whole.slice(2, -2) });
    } else if (match[3] !== undefined) {
      out.push({ kind: "em", text: whole.slice(1, -1) });
    } else if (match[5] !== undefined) {
      const close = whole.indexOf("](");
      out.push({ kind: "link", text: whole.slice(1, close), href: match[5] });
    }
    last = at + whole.length;
  }
  if (last < text.length) {
    out.push({ kind: "text", text: text.slice(last) });
  }
  return out;
}

function cells(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

function isRule(line: string): boolean {
  return /^\|?\s*:?-{2,}/.test(line.trim());
}

export function blocks(text: string): Block[] {
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  const out: Block[] = [];
  let paragraph: string[] = [];
  const flush = () => {
    if (paragraph.length > 0) {
      out.push({ kind: "paragraph", inline: inline(paragraph.join(" ")) });
      paragraph = [];
    }
  };
  let i = 0;
  while (i < lines.length) {
    const line = lines[i] ?? "";
    const fence = /^```(\w*)/.exec(line);
    if (fence !== null) {
      flush();
      const body: string[] = [];
      i += 1;
      while (i < lines.length && !(lines[i] ?? "").startsWith("```")) {
        body.push(lines[i] ?? "");
        i += 1;
      }
      out.push({ kind: "code", lang: fence[1] ?? "", text: body.join("\n") });
      i += 1;
      continue;
    }
    const heading = /^(#{1,6})\s+(.*)$/.exec(line);
    if (heading !== null) {
      flush();
      out.push({
        kind: "heading",
        level: (heading[1] ?? "#").length,
        inline: inline(heading[2] ?? ""),
      });
      i += 1;
      continue;
    }
    if (line.trim().startsWith("|") && i + 1 < lines.length && isRule(lines[i + 1] ?? "")) {
      flush();
      const rows: string[][] = [cells(line)];
      i += 2;
      while (i < lines.length && (lines[i] ?? "").trim().startsWith("|")) {
        rows.push(cells(lines[i] ?? ""));
        i += 1;
      }
      out.push({ kind: "table", rows });
      continue;
    }
    const bullet = /^\s*(?:[-*+]|\d+[.)])\s+(.*)$/.exec(line);
    if (bullet !== null) {
      flush();
      const ordered = /^\s*\d/.test(line);
      const items: Inline[][] = [];
      while (i < lines.length) {
        const item = /^\s*(?:[-*+]|\d+[.)])\s+(.*)$/.exec(lines[i] ?? "");
        if (item === null) break;
        items.push(inline(item[1] ?? ""));
        i += 1;
      }
      out.push({ kind: "list", ordered, items });
      continue;
    }
    if (line.startsWith(">")) {
      flush();
      const quoted: string[] = [];
      while (i < lines.length && (lines[i] ?? "").startsWith(">")) {
        quoted.push((lines[i] ?? "").replace(/^>\s?/, ""));
        i += 1;
      }
      out.push({ kind: "quote", inline: inline(quoted.join(" ")) });
      continue;
    }
    if (line.trim() === "") {
      flush();
    } else {
      paragraph.push(line.trim());
    }
    i += 1;
  }
  flush();
  return out;
}
