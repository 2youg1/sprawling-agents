// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { dependencies } from "../../../../package.json";

export type Drawn = "pdf" | "docx" | "html";

const BY_EXTENSION: Readonly<Record<string, Drawn>> = {
  pdf: "pdf",
  docx: "docx",
  html: "html",
  htm: "html",
};

export const NAME: Readonly<Record<Drawn, string>> = { pdf: "PDF", docx: "DOCX", html: "HTML" };

export const TOOL: Readonly<Record<Exclude<Drawn, "html">, string>> = {
  pdf: `pdf.js ${dependencies["pdfjs-dist"]}`,
  docx: `docx-preview ${dependencies["docx-preview"]}`,
};

export function drawnAs(path: string): Drawn | null {
  const name = path.slice(path.lastIndexOf("/") + 1);
  const dot = name.lastIndexOf(".");
  return dot <= 0 ? null : (BY_EXTENSION[name.slice(dot + 1).toLowerCase()] ?? null);
}
export function markdownFile(name: string): boolean {
  return /\.(?:md|markdown|mdown|mkd)$/iu.test(name);
}
