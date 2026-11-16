// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";
import type { Attachment } from "svelte/attachments";

import type { CopyProps } from "./copy";

export type Ink = "plain" | "comment" | "string" | "number" | "word";

export interface Piece {
  readonly ink: Ink;
  readonly text: string;
}

export function painted(text: string, source: string): Promise<readonly Piece[]> {
  return import("./paint").then(
    (highlighter) => highlighter.paint(text, source),
    (): readonly Piece[] => [{ ink: "plain", text }],
  );
}

export interface InkedLook {
  readonly pieces: readonly Piece[];
}

export interface Crumb {
  readonly part: string;
  readonly last: boolean;
}

export interface CodeLook {
  readonly crumbs: readonly Crumb[];

  readonly trail: { readonly "aria-label": string };

  readonly gutter: string;
  readonly text: string;
  readonly path: string;

  readonly cited: number | undefined;
  readonly copy: CopyProps;
  readonly scroller: Attachment<HTMLElement>;
  readonly mark: Attachment<HTMLElement>;
  readonly aside: Snippet | undefined;
}

export function crumbsOf(path: string): readonly Crumb[] {
  const parts = path.split("/").filter((part) => part !== "");
  return parts.map((part, at) => ({ part, last: at === parts.length - 1 }));
}

export function gutterOf(text: string): string {
  return text
    .split("\n")
    .map((_line, at) => String(at + 1))
    .join("\n");
}
