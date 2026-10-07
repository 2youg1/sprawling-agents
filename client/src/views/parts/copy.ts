// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one press of a copy key does, and what the key says afterwards
// (client/Spec.lean §4-65). Every press-to-copy key in the client is
// `copy.svelte`, which seats this wiring and draws `copy.look.svelte`,
// so the receipt, its length and the reason a write did not land have
// one author here.
//
// The receipt appears only after the write landed, so a press that
// failed never shows a lying check; a write the browser refused says
// why, because the person is about to paste and needs to know the
// clipboard still holds whatever it held before.

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import type { GlyphName } from "./glyph";

// How long the receipt holds its check mark: long enough to see one,
// short enough that the mark never becomes the key's face.
export const RECEIPT_MS = 1200;

// Where a copy key stands after its last press. A refusal holds until
// the next press, because its reason is something to read, not a
// flash.
export type Copied =
  | { readonly kind: "rest" }
  | { readonly kind: "copied" }
  | { readonly kind: "refused"; readonly why: Refusal };

// Why a write did not land: the page has no clipboard at all (the
// browser offers one only to a secure context - https or localhost),
// or the browser refused the write and said why in its own words.
export type Refusal = { readonly kind: "absent" } | { readonly kind: "refused"; readonly said: string };

export const REST: Copied = { kind: "rest" };

// The clipboard as this part uses it. `navigator.clipboard` is
// `undefined` outside a secure context, whatever its type says.
export interface Board {
  writeText(text: string): Promise<void>;
}

// Waits `ms` and runs `run`, returning what cancels the wait.
export type Later = (run: () => void, ms: number) => () => void;

export interface Hands {
  readonly board: () => Board | undefined;
  readonly later: Later;
  // The key's new state; the seat holds it.
  readonly show: (copied: Copied) => void;
}

// One copy key's presses. Each press writes and shows what the write
// came to; a receipt returns to rest `RECEIPT_MS` later, and a press
// before then restarts that wait. A write that lands after a later
// press was made is dropped, so a slow first write never overwrites the
// answer to the second.
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

// The write itself, settled either way: the promise never rejects, so
// no caller can drop the reason a write failed.
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

// A key that says its name beside its mark, against one that shows the
// mark alone and carries the name for a screen reader: a command row
// has room for the word, a code view's header corner does not.
export type CopyForm = "named" | "bare";

// What the button a look draws carries, spread onto it whole.
export interface KeyWire {
  readonly type: "button";
  readonly "aria-label": string | undefined;
  readonly onclick: () => void;
}

// What every look of a copy key is given, and nothing else.
export interface CopyLook {
  readonly form: CopyForm;
  // "copy", "copied" or "not copied", in the reader's language: the
  // visible name of a named key, the accessible name of both forms.
  readonly name: string;
  readonly face: GlyphName;
  readonly refused: boolean;
  // The hint under the key: what lands on the clipboard, or, after a
  // refusal, why nothing did.
  readonly note: string;
  // The same outcome for a screen reader, said once as it happens; the
  // empty string at rest. `heard` goes on the element that holds it.
  readonly said: string;
  readonly heard: HeardWire;
  readonly key: KeyWire;
}

// A polite live region, so the receipt is announced without moving
// focus.
export interface HeardWire {
  readonly role: "status";
}

export interface CopyProps {
  // Exactly what lands on the clipboard; a function when the words are
  // only final at the moment of the press.
  readonly text: string | (() => string);
  // What the hint says lands on the clipboard. Absent, the hint quotes
  // the text itself, which suits a command and not a whole file.
  readonly note?: string | undefined;
  readonly form?: CopyForm | undefined;
}

export function textOf(text: CopyProps["text"]): string {
  return typeof text === "string" ? text : text();
}

// The whole look, from the props, the key's state and the press.
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
