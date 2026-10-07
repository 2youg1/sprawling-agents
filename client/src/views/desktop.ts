// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The windows on this person's own machine a building's connector may
// touch, one building at a time: where that file lives, and the whole
// value the allowlist editor's look draws (`DesktopLook`), with every
// word translated and the box's and the save's handlers inside wire
// bags (client D95).
//
// One box holding the whole file, because that is what the file is: the
// connector reads it whole at start-up and permits nothing it cannot
// read, so a form with a field per window would be a second reading of
// a syntax this side does not own (`crates/city/Spec.lean` §8-26).

import { fill, say } from "../core/lang";
import type { Lang } from "../core/lang";
import { Address } from "../wire";

// Where the file lives, as the city spells it. One authority on this
// side too: a page that joined its own path could join one that leaves
// the subtree.
export function desktopScopeAt(addr: Address): Address {
  return Address.make(`${addr}/.sprawling/DESKTOP.toml`);
}

// What the editor knows: the text in the box, whether somebody typed
// it, and what the city answered for the file.
export interface DesktopState {
  readonly addr: Address;
  readonly draft: string;
  readonly edited: boolean;
  // `missing`: the city answered `unavailable`, for a building with no
  // allowlist yet or a file it could not read; `empty`: it holds a file
  // with nothing in it; `held`: anything else, including still asking.
  readonly file: "missing" | "empty" | "held";
}

// The bag spread on the box: its name, its hint, its text, and what
// typing in it does.
export interface BoxWire {
  readonly "aria-label": string;
  readonly placeholder: string;
  readonly spellcheck: false;
  readonly value: string;
  readonly oninput: (event: { readonly currentTarget: { readonly value: string } }) => void;
}

export interface DesktopLook {
  readonly box: BoxWire;
  // What is missing and how a save creates it; absent unless the city
  // could not answer the file.
  readonly missing: string | undefined;
  // The one press: the reason it cannot be used is present while
  // nothing in the box changed.
  readonly save: { readonly label: string; readonly why: string | undefined; readonly press: () => void };
  // Said beside the save while an empty file is shown unchanged.
  readonly none: string | undefined;
  readonly scope: Address;
}

export interface Hands {
  readonly type: (text: string) => void;
  readonly save: () => void;
}

export function lookOf(state: DesktopState, lang: Lang, hands: Hands): DesktopLook {
  const scope = desktopScopeAt(state.addr);
  return {
    box: {
      "aria-label": say(lang, "desktop_allowlist"),
      placeholder: say(lang, "desktop_empty"),
      spellcheck: false,
      value: state.draft,
      oninput: (event) => {
        hands.type(event.currentTarget.value);
      },
    },
    missing: state.file === "missing" ? fill(say(lang, "desktop_missing"), { path: scope }) : undefined,
    save: { label: say(lang, "desktop_save"), why: state.edited ? undefined : say(lang, "desktop_unchanged"), press: hands.save },
    none: !state.edited && state.file === "empty" ? say(lang, "desktop_none") : undefined,
    scope,
  };
}
