// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The formats RefRain draws as a page besides its text, and the tool
// that draws each (client-SPEC 4-54). The wire's `Format` says only
// whether a text is Markdown, so which drawing a file gets is read off
// its name here, once; the tool's version is the one `package.json`
// pins, so the label on a preview cannot name a version the bundle does
// not carry.

import { dependencies } from "../../../../package.json";

export type Drawn = "pdf" | "docx" | "html";

const BY_EXTENSION: Readonly<Record<string, Drawn>> = {
  pdf: "pdf",
  docx: "docx",
  html: "html",
  htm: "html",
};

// How a format is named on the screen: the same three letters in every
// language, as a person sees them on the file.
export const NAME: Readonly<Record<Drawn, string>> = { pdf: "PDF", docx: "DOCX", html: "HTML" };

// The library that draws a format and the version the bundle carries;
// HTML is drawn by the browser itself, which has no version to state.
export const TOOL: Readonly<Record<Exclude<Drawn, "html">, string>> = {
  pdf: `pdf.js ${dependencies["pdfjs-dist"]}`,
  docx: `docx-preview ${dependencies["docx-preview"]}`,
};

export function drawnAs(path: string): Drawn | null {
  const name = path.slice(path.lastIndexOf("/") + 1);
  const dot = name.lastIndexOf(".");
  return dot <= 0 ? null : (BY_EXTENSION[name.slice(dot + 1).toLowerCase()] ?? null);
}
