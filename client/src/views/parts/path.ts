// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one printed path decides before anything is drawn: how much of
// it is shown, whether the text opens it inside the page, and whether
// the reveal key can hand it to the desktop's file manager.
//
// **A path is one line.** It is an identifier, and the rule this client
// settled on for identifiers is the single-line ellipsis: breaking it
// after a `/` let the engine break inside a segment when one did not
// fit, and the value a person came to read became two lines of which
// neither was the path. What a person already knows is the prefix, so
// `base` cuts that end and the reveal key is how the whole value is
// reached.
//
// The reveal key asks the city to hand the path to the file manager. A
// path this build cannot turn into an address stays text with the key
// saying why (client/Spec.lean §7-2): the key keeps its seat in the Tab
// order under `aria-disabled`, so a keyboard reaches it and a screen
// reader reads the reason through the hint, and pressing it lands in a
// no-op rather than in a surprise.

import { Option, Schema } from "effect";

import { Address } from "../../wire";
import type { IconButtonProps } from "./icon_button";

export interface PathProps {
  // As the city spells it: relative to the city, never to a disk.
  readonly path: string;
  // The part of the path the reader is already looking at, cut from the
  // front of what is drawn.
  readonly base?: string | undefined;
  // Absent leaves the path as text, which is what a path this build
  // cannot open should look like.
  readonly onOpen?: (() => void) | undefined;
}

// The two sentences the reveal key says, in the person's language.
export interface PathWords {
  readonly reveal: string;
  readonly inert: string;
}

// Spread on the `<button>` the path's text becomes when it opens.
export interface OpenWire {
  readonly type: "button";
  readonly onclick: () => void;
}

export interface PathLook {
  readonly shown: string;
  // Present means the text is a button that opens the path in the page.
  readonly open: OpenWire | undefined;
  // The props of the reveal key (`icon_button.svelte`).
  readonly reveal: IconButtonProps;
}

export function shownOf(path: string, base: string | undefined): string {
  return base !== undefined && path.startsWith(`${base}/`) ? path.slice(base.length + 1) : path;
}

export function lookOf(props: PathProps, words: PathWords, reveal: (address: Address) => void): PathLook {
  const { onOpen } = props;
  const address = Schema.decodeOption(Address)(props.path);
  return {
    shown: shownOf(props.path, props.base),
    open: onOpen === undefined ? undefined : { type: "button", onclick: () => { onOpen(); } },
    reveal: Option.match(address, {
      onNone: () => ({ glyph: "reveal", label: words.reveal, why: words.inert }),
      onSome: (value) => ({ glyph: "reveal", label: words.reveal, onPress: () => { reveal(value); } }),
    }),
  };
}
