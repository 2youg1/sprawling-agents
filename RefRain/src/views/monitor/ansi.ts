// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export type Tone = "plain" | "trouble" | "progress" | "strong" | "aside";

export interface Span {
  readonly tone: Tone;
  readonly bold: boolean;
  readonly text: string;
}

const ESC = String.fromCharCode(27);
const SGR = new RegExp(`${ESC}\\[([0-9;]*)m`, "g");
const OTHER = new RegExp(`${ESC}\\[[0-9;?]*[A-Za-z]`, "g");

function toneOf(code: number, now: Tone): Tone {
  if (code === 0 || code === 39) return "plain";
  if ([31, 33, 35, 91, 93, 95].includes(code)) return "trouble";
  if ([32, 34, 36, 92, 94, 96].includes(code)) return "progress";
  if (code === 37 || code === 97) return "strong";
  if (code === 30 || code === 90 || code === 2) return "aside";
  return now;
}

export function spans(text: string): readonly Span[] {
  const out: Span[] = [];
  let tone: Tone = "plain";
  let bold = false;
  let from = 0;
  const push = (upto: number): void => {
    const piece = text.slice(from, upto).replace(OTHER, "");
    if (piece !== "") out.push({ tone, bold, text: piece });
  };
  for (const match of text.matchAll(SGR)) {
    push(match.index);
    for (const part of (match[1] ?? "").split(";")) {
      const code = part === "" ? 0 : Number(part);
      if (code === 0) {
        tone = "plain";
        bold = false;
      } else if (code === 1) {
        bold = true;
      } else if (code === 22) {
        bold = false;
      } else {
        tone = toneOf(code, tone);
      }
    }
    from = match.index + match[0].length;
  }
  push(text.length);
  return out;
}

export function rows(text: string): readonly (readonly Span[])[] {
  const out: Span[][] = [[]];
  for (const span of spans(text)) {
    const parts = span.text.split("\n");
    for (const [at, part] of parts.entries()) {
      if (at > 0) out.push([]);
      if (part !== "") out.at(-1)?.push({ tone: span.tone, bold: span.bold, text: part });
    }
  }
  return out;
}

export const INK: Readonly<Record<Tone, string>> = {
  plain: "text-text-quiet",
  trouble: "text-alert",
  progress: "text-accent",
  strong: "text-text",
  aside: "text-text-faint",
};
