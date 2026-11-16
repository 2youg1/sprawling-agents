// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { renderAsync } from "docx-preview";
import type { Options } from "docx-preview";

import { unpack } from "./zip";

const W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";


export interface Coverage {
  readonly revisions: number;
  readonly comments: number;
  readonly objects: number;
}

export type Read =
  | { readonly kind: "drawn"; readonly page: string; readonly coverage: Coverage }
  | { readonly kind: "refused"; readonly why: "too_large" | "shape" }
  | { readonly kind: "broken"; readonly reason: string };


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


function boardOf(gutter: number): string {
  const px = `${String(gutter)}px`;
  return `.docx-wrapper { background: transparent; padding: ${px}; align-items: safe center; } .docx-wrapper > section.docx { margin-bottom: ${px}; }`;
}


function gutterOf(doc: Document): number {
  const probe = doc.createElement("div");
  probe.style.padding = "var(--spacing-pane)";
  doc.body.append(probe);
  const px = Number.parseFloat(getComputedStyle(probe).paddingTop);
  probe.remove();
  return Number.isFinite(px) ? px : 0;
}


const FIT_STEP = 40;
const FIT_MIN = 280;

export function readDocx(bytes: Uint8Array): Promise<Read> {
  return unpack(bytes).then((unpacked) =>
    unpacked.kind === "refused" ? unpacked : drawn(bytes, unpacked.parts),
  );
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


function widthOf(body: HTMLElement): number | null {
  const stated = body
    .querySelector("section.docx")
    ?.getAttribute("style")
    ?.match(/(?:^|;)\s*width:\s*([\d.]+)pt/);
  return stated?.[1] === undefined ? null : (Number(stated[1]) * 4) / 3;
}


function fitted(width: number | null, gutter: number): string {
  if (width === null) return "";
  const board = 2 * gutter;
  const rules: string[] = [];
  for (
    let frame = Math.ceil((width + board) / FIT_STEP) * FIT_STEP;
    frame > FIT_MIN;
    frame -= FIT_STEP
  ) {
    const zoom = Math.min(1, (frame - FIT_STEP - board) / width);
    rules.push(
      `@media (max-width: ${String(frame)}px) { .docx-wrapper > section.docx { zoom: ${zoom.toFixed(3)}; } }`,
    );
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
      count(body, /<w:(object|altChunk|control)\b/g) +
      [...parts.keys()].filter((name) => name.startsWith("word/embeddings/")).length,
  };
}

function textOf(parts: ReadonlyMap<string, Uint8Array>, name: string): string {
  const part = parts.get(name);
  return part === undefined ? "" : new TextDecoder().decode(part);
}


export function docxLines(bytes: Uint8Array): Promise<readonly string[] | null> {
  return unpack(bytes).then((unpacked) => {
    if (unpacked.kind === "refused") return null;
    const xml = new DOMParser().parseFromString(
      textOf(unpacked.parts, "word/document.xml"),
      "application/xml",
    );
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
      lines[line] =
        (lines[line] ?? "") + (own === "t" ? child.textContent : own === "tab" ? "\t" : "\n");
    } else {
      walk(child, line, lines);
    }
  }
}
