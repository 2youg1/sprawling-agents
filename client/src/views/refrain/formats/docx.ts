// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A DOCX read for a page (client/Spec.lean §4-54, client D32): its archive checked
// and unpacked within bounds first (`zip.ts`), then laid out as pages by
// docx-preview into elements this page never shows, and handed on as
// one HTML document for the sandboxed frame, so nothing the file holds
// lands in the client's own page. What the drawing leaves out is
// counted from the file itself, so a revision or a comment the preview
// does not show is said rather than read as absent. Its own lazy chunk.

import { renderAsync } from "docx-preview";
import type { Options } from "docx-preview";

import { unpack } from "./zip";

const W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

// What the file holds that the drawing does not show as itself.
export interface Coverage {
  readonly revisions: number;
  readonly comments: number;
  readonly objects: number;
}

export type Read =
  | { readonly kind: "drawn"; readonly page: string; readonly coverage: Coverage }
  | { readonly kind: "refused"; readonly why: "too_large" | "shape" }
  | { readonly kind: "broken"; readonly reason: string };

// Laid out as the file's pages; pictures and embedded faces travel as
// `data:` addresses, the only kind the frame loads; a part of foreign
// HTML (`altChunk`) is not drawn, because docx-preview would draw it in
// a frame of its own with no sandbox.
const OPTIONS: Partial<Options> = {
  className: "docx",
  inWrapper: true,
  breakPages: true,
  ignoreLastRenderedPageBreak: true,
  useBase64URL: true,
  renderAltChunks: false,
  renderChanges: false,
  renderComments: false,
  experimental: false,
};

// The pages stand on the pane behind the frame, not on the library's
// grey board, and a page wider than the frame starts at its left edge
// rather than spilling past both. The space around the pages and
// between them is the pane's gutter, `--spacing-pane`, at the density
// the person chose; the frame's page cannot read the theme, so the
// board is written with the pixels the token resolves to here.
function boardOf(gutter: number): string {
  const px = `${String(gutter)}px`;
  return `.docx-wrapper { background: transparent; padding: ${px}; align-items: safe center; } .docx-wrapper > section.docx { margin-bottom: ${px}; }`;
}

// The pane's gutter in pixels, as the theme resolves it on this page.
function gutterOf(doc: Document): number {
  const probe = doc.createElement("div");
  probe.style.padding = "var(--spacing-pane)";
  doc.body.append(probe);
  const px = Number.parseFloat(getComputedStyle(probe).paddingTop);
  probe.remove();
  return Number.isFinite(px) ? px : 0;
}

// A page wider than the frame is scaled to the frame in steps of this
// many pixels, so a line still breaks where the file breaks it and
// nothing is cut off at the side; the frame's own width is all a rule
// inside it can ask.
const FIT_STEP = 40;
const FIT_MIN = 280;

export function readDocx(bytes: Uint8Array): Promise<Read> {
  return unpack(bytes).then((unpacked) => (unpacked.kind === "refused" ? unpacked : drawn(bytes, unpacked.parts)));
}

function drawn(bytes: Uint8Array, parts: ReadonlyMap<string, Uint8Array>): Promise<Read> {
  const body = document.createElement("div");
  const styles = document.createElement("div");
  const gutter = gutterOf(document);
  const drawing: Promise<unknown> = renderAsync(new Blob([bytes.slice()]), body, styles, OPTIONS);
  return drawing.then(
    (): Read => ({
      kind: "drawn",
      page: `<!doctype html><html><head>${styles.innerHTML}<style>${boardOf(gutter)}${fitted(widthOf(body), gutter)}</style></head><body>${body.innerHTML}</body></html>`,
      coverage: coverageOf(parts),
    }),
    (reason: unknown): Read => ({ kind: "broken", reason: String(reason) }),
  );
}

// The first page's width in CSS pixels, as docx-preview set it in
// points, or `null` when it set none.
function widthOf(body: HTMLElement): number | null {
  const stated = body.querySelector("section.docx")?.getAttribute("style")?.match(/(?:^|;)\s*width:\s*([\d.]+)pt/);
  return stated?.[1] === undefined ? null : (Number(stated[1]) * 4) / 3;
}

// One rule per step of frame width below the page's, the narrowest
// last so it wins: each scales the page to the narrow end of its step,
// less the gutter on either side.
function fitted(width: number | null, gutter: number): string {
  if (width === null) return "";
  const board = 2 * gutter;
  const rules: string[] = [];
  for (let frame = Math.ceil((width + board) / FIT_STEP) * FIT_STEP; frame > FIT_MIN; frame -= FIT_STEP) {
    const zoom = Math.min(1, (frame - FIT_STEP - board) / width);
    rules.push(`@media (max-width: ${String(frame)}px) { .docx-wrapper > section.docx { zoom: ${zoom.toFixed(3)}; } }`);
  }
  return rules.join(" ");
}

function coverageOf(parts: ReadonlyMap<string, Uint8Array>): Coverage {
  const body = textOf(parts, "word/document.xml");
  const count = (text: string, pattern: RegExp): number => text.match(pattern)?.length ?? 0;
  return {
    revisions: count(body, /<w:(ins|del|moveFrom|moveTo)\b/g),
    comments: count(textOf(parts, "word/comments.xml"), /<w:comment\b/g),
    objects:
      count(body, /<w:(object|altChunk|control)\b/g) + [...parts.keys()].filter((name) => name.startsWith("word/embeddings/")).length,
  };
}

function textOf(parts: ReadonlyMap<string, Uint8Array>, name: string): string {
  const part = parts.get(name);
  return part === undefined ? "" : new TextDecoder().decode(part);
}

// The body's paragraphs as their tracked changes leave them, one line
// each, a paragraph inside another (a text box) on a line of its own:
// what two versions of a DOCX are compared by. Deleted text is spelled
// `w:delText`, so reading only `w:t` reads the result.
export function docxLines(bytes: Uint8Array): Promise<readonly string[] | null> {
  return unpack(bytes).then((unpacked) => {
    if (unpacked.kind === "refused") return null;
    const xml = new DOMParser().parseFromString(textOf(unpacked.parts, "word/document.xml"), "application/xml");
    const lines: string[] = [];
    walk(xml.documentElement, null, lines);
    return lines;
  });
}

function walk(element: Element, line: number | null, lines: string[]): void {
  for (const child of element.children) {
    const own = child.namespaceURI === W ? child.localName : "";
    if (own === "p") {
      lines.push("");
      walk(child, lines.length - 1, lines);
    } else if (line !== null && (own === "t" || own === "tab" || own === "br" || own === "cr")) {
      lines[line] = (lines[line] ?? "") + (own === "t" ? child.textContent : own === "tab" ? "\t" : "\n");
    } else {
      walk(child, line, lines);
    }
  }
}
