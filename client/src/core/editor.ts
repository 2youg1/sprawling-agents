// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export type Editor = "none" | "vscode" | "vscode-insiders";

export interface Opening {
  readonly editor: Editor;
  readonly folder: string;
  readonly path: string;
  readonly line: number;
}

export function editorLink(at: Opening): string | null {
  return at.editor === "none" ? null : `${at.editor}://file/${at.folder}/${at.path}:${String(at.line)}:1`;
}
