// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import type { GlyphName } from "./glyph";

export const RECEIPT_MS = 1200;

export type Copied =
  | { readonly kind: "rest" }
  | { readonly kind: "copied" }
  | { readonly kind: "refused"; readonly why: Refusal };

export type Refusal =
  { readonly kind: "absent" } | { readonly kind: "refused"; readonly said: string };

export const REST: Copied = { kind: "rest" };

export interface Board {
  writeText(text: string): Promise<void>;
}

export type Later = (run: () => void, ms: number) => () => void;

export interface Hands {
  readonly board: () => Board | undefined;
  readonly later: Later;

  readonly show: (copied: Copied) => void;
}

export function presser(hands: Hands): (text: string) => void {
  let pressed = 0;
  let cancel: (() => void) | undefined = undefined;
  return (text) => {
    pressed += 1;
    const mine = pressed;
    void written(hands.board(), text).then((copied) => {
      if (mine !== pressed) return;
      cancel?.();
      cancel = undefined;
      hands.show(copied);
      if (copied.kind === "copied") {
        cancel = hands.later(() => {
          cancel = undefined;
          hands.show(REST);
        }, RECEIPT_MS);
      }
    });
  };
}

export function written(board: Board | undefined, text: string): Promise<Copied> {
  if (board === undefined) return Promise.resolve({ kind: "refused", why: { kind: "absent" } });
  return board.writeText(text).then(
    (): Copied => ({ kind: "copied" }),
    (cause: unknown): Copied => ({
      kind: "refused",
      why: { kind: "refused", said: cause instanceof Error ? cause.message : String(cause) },
    }),
  );
}

export type CopyForm = "named" | "bare";

export interface KeyWire {
  readonly type: "button";
  readonly "aria-label": string | undefined;
  readonly onclick: () => void;
}

export interface CopyLook {
  readonly form: CopyForm;

  readonly name: string;
  readonly face: GlyphName;
  readonly refused: boolean;

  readonly note: string;

  readonly said: string;
  readonly heard: HeardWire;
  readonly key: KeyWire;
}

export interface HeardWire {
  readonly role: "status";
}

export interface CopyProps {
  readonly text: string | (() => string);

  readonly note?: string | undefined;
  readonly form?: CopyForm | undefined;
}

export function textOf(text: CopyProps["text"]): string {
  return typeof text === "string" ? text : text();
}

export function lookOf(props: CopyProps, copied: Copied, lang: Lang, press: () => void): CopyLook {
  const form = props.form ?? "named";
  const name = say(lang, NAME[copied.kind]);
  const resting = props.note ?? fill(say(lang, "copy_note"), { text: textOf(props.text) });
  return {
    form,
    name,
    face: FACE[copied.kind],
    refused: copied.kind === "refused",
    note: copied.kind === "refused" ? reasonOf(lang, copied.why) : resting,
    said: copied.kind === "rest" ? "" : name,
    heard: { role: "status" },
    key: { type: "button", "aria-label": form === "bare" ? name : undefined, onclick: press },
  };
}

function reasonOf(lang: Lang, why: Refusal): string {
  switch (why.kind) {
    case "absent":
      return say(lang, "copy_no_clipboard");
    case "refused":
      return fill(say(lang, "copy_refused_note"), { why: why.said });
  }
}

const NAME: Record<Copied["kind"], Key> = {
  rest: "setup_copy",
  copied: "setup_copied",
  refused: "copy_refused",
};

const FACE: Record<Copied["kind"], GlyphName> = {
  rest: "copy",
  copied: "check",
  refused: "cross",
};
