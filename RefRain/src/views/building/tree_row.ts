// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export type TreeMark =
  | { readonly kind: "directory"; readonly open: boolean }
  | { readonly kind: "transcript"; readonly hint: string }
  | { readonly kind: "file" };

export type TreeTone = "picked" | "hidden" | "plain";

export interface TreeRowWire {
  readonly type: "button";
  readonly "aria-expanded": boolean | undefined;
  readonly onclick: () => void;
}

export interface TreeRowLook {
  readonly name: string;
  readonly mark: TreeMark;
  readonly tone: TreeTone;
  readonly coded: boolean;
  readonly live: string | undefined;
  readonly size: string | undefined;
  readonly wire: TreeRowWire;
}

export function treeRowWire(mark: TreeMark, press: () => void): TreeRowWire {
  return {
    type: "button",
    "aria-expanded": mark.kind === "directory" ? mark.open : undefined,
    onclick: press,
  };
}
