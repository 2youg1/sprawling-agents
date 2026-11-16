// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../core/lang";
import type { Lang } from "../core/lang";
import { Address } from "../wire";

export function desktopScopeAt(addr: Address): Address {
  return Address.make(`${addr}/.sprawling/DESKTOP.toml`);
}

export interface DesktopState {
  readonly addr: Address;
  readonly draft: string;
  readonly edited: boolean;

  readonly file: "missing" | "empty" | "held";
}

export interface BoxWire {
  readonly "aria-label": string;
  readonly placeholder: string;
  readonly spellcheck: false;
  readonly value: string;
  readonly oninput: (event: { readonly currentTarget: { readonly value: string } }) => void;
}

export interface DesktopLook {
  readonly box: BoxWire;

  readonly missing: string | undefined;

  readonly save: {
    readonly label: string;
    readonly why: string | undefined;
    readonly press: () => void;
  };

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
    missing:
      state.file === "missing" ? fill(say(lang, "desktop_missing"), { path: scope }) : undefined,
    save: {
      label: say(lang, "desktop_save"),
      why: state.edited ? undefined : say(lang, "desktop_unchanged"),
      press: hands.save,
    },
    none: !state.edited && state.file === "empty" ? say(lang, "desktop_none") : undefined,
    scope,
  };
}
